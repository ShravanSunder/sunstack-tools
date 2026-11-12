// Model-backed runs through the real addon and Rust executor. Only model answers
// come from the controlled loopback backend; this is not hosted-provider proof.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { test } from 'node:test';

import type { Configuration, RunOutcome } from '../src/index.ts';
import { check, load, run } from '../src/index.ts';
import type { NativeBinding } from '../src/native-binding.ts';
import { installedBindingSources, resolveBinding } from '../src/native-binding.ts';
import { createSdkCalls } from '../src/sdk-calls.ts';
import type { CapturedRequest, ControlledBackend } from './support/controlled-backend.ts';
import { answerChoice, answerNoul, startControlledBackend } from './support/controlled-backend.ts';
import { withoutWritableDirectories } from './support/read-only-directories.ts';
import type { OwnedDirectory } from './support/sdk-fixtures.ts';
import {
	expectedInlineSources,
	inlineConfiguration,
	ownedDirectory,
	ruleEntries,
	rulebookMarkdown,
	writeFileProject,
	writeFiles,
	writeSkillRepository,
} from './support/sdk-fixtures.ts';

// A synthetic key for the loopback backend; the credential store stays off.
process.env['OPENROUTER_API_KEY'] = 'synthetic-controlled-key';
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

function plain(value: unknown): unknown {
	return JSON.parse(
		JSON.stringify(value, (_key, item: unknown) => (typeof item === 'bigint' ? `${item}n` : item)),
	);
}

function canonicalBodies(bodies: readonly unknown[]): string[] {
	return bodies.map((body) => JSON.stringify(body)).toSorted();
}

function describe(outcome: RunOutcome): string {
	return JSON.stringify(plain(outcome)).slice(0, 2000);
}

/** Answer filter questions by folder and rule questions with "no", for one backend's lifetime. */
function answerSkillRequests(owned: ControlledBackend): void {
	const answerNext = async (): Promise<void> => {
		const request = await owned.nextRequest();
		const questions = request.body['questions'];
		if (typeof questions === 'object' && questions !== null && 'purpose' in questions) {
			answerChoice(
				request,
				JSON.stringify(request.body).includes('alpha-portable') ? 'portable' : 'local',
				['local', 'portable'],
			);
		} else {
			answerNoul(request, 0.1);
		}
		await answerNext();
	};
	void answerNext();
}

// Tests run in order; these fixtures are released after the last one.
const sharedBackend: ControlledBackend = await startControlledBackend();
const repository: OwnedDirectory = ownedDirectory('skills');
writeSkillRepository(repository.path);

await test('complete inline authoring runs without definition files and equals the file fixture', async () => {
	const emptyBase = ownedDirectory('empty-base');
	const project = ownedDirectory('project');
	const backend = await startControlledBackend();
	answerSkillRequests(backend);
	try {
		const firstRequest = backend.received.length;
		const inline = await withoutWritableDirectories([emptyBase.path, repository.path], () =>
			run(inlineConfiguration(emptyBase.path, backend.endpoint), {
				root: repository.path,
				scope: { kind: 'all' },
			}),
		);
		const inlineRequests = backend.received.slice(firstRequest).map((request) => request.body);

		writeFileProject(project.path, backend.endpoint);
		const loaded = await load({ directory: project.path });
		assert.equal(loaded.kind, 'loaded');
		if (loaded.kind !== 'loaded') return;
		const fileStart = backend.received.length;
		const file = await run(loaded.value, { root: repository.path, scope: { kind: 'all' } });
		const fileRequests = backend.received.slice(fileStart).map((request) => request.body);

		assert.equal(inline.kind, 'completed', describe(inline));
		assert.equal(file.kind, 'completed', describe(file));
		if (inline.kind !== 'completed' || file.kind !== 'completed') return;
		// A warn-severity finding leaves the engine status clean, as in the CLI.
		assert.equal(inline.report.status.kind, 'clean');
		assert.equal(inline.report.counts.warn, 1);
		// The filter gated the rule to the reusable folder only.
		assert.deepEqual(
			inline.report.filter_answers.map((answer) => plain(answer)),
			file.report.filter_answers.map((answer) => plain(answer)),
		);
		assert.ok(inline.report.findings.length > 0);
		assert.ok(inline.report.findings.every((finding) => finding.file.startsWith('skills/alpha/')));
		assert.ok(inline.report.findings.every((finding) => finding.message === 'Name the actor.'));

		// Same model requests, in any order.
		assert.ok(inlineRequests.length >= 3, 'two filter folders and at least one rule request');
		assert.deepEqual(canonicalBodies(inlineRequests), canonicalBodies(fileRequests));

		// Same semantic report; only declared origin representations differ.
		const semantic = (report: typeof inline.report): unknown => {
			const { configuration: _configuration, sources: _sources, findings, ...rest } = report;
			const withoutStandard = findings.map((finding) =>
				Object.assign({}, finding, { related: finding.related.slice(1) }),
			);
			return plain({ ...rest, findings: withoutStandard });
		};
		assert.deepEqual(semantic(inline.report), semantic(file.report));
		assert.equal(inline.report.findings[0]?.related[0]?.location.kind, 'definition');
		assert.deepEqual(
			inline.report.sources.map((source) =>
				source.origin.kind === 'inline' ? source.origin.id : source.origin.kind,
			),
			[
				'instruction-standards',
				'ownership-standards',
				'instruction-standards#/rulebooks/0/entries',
				'instruction-standards#/evidence/utilities/paragraph-node',
				'instruction-standards#/evidence/matchers/paragraph',
			],
		);
		// Each inline source is the exact admitted value, digested as reported.
		for (const source of inline.report.sources) {
			assert.equal(source.sha256, createHash('sha256').update(source.content).digest('hex'));
			assert.equal(source.acquisition, 'inline');
		}
		const [root, markdown, entries, utility, matcher] = inline.report.sources;
		assert.deepEqual(
			inline.report.sources.map((source) => source.representation),
			[
				'normalized_value',
				'original_text',
				'normalized_value',
				'normalized_value',
				'normalized_value',
			],
		);
		assert.equal(markdown?.content, rulebookMarkdown);
		// The whole admitted value of every normalized source, against the declared contract.
		const expected = expectedInlineSources(backend.endpoint);
		assert.deepEqual(JSON.parse(root?.content ?? 'null'), expected.root);
		assert.deepEqual(JSON.parse(entries?.content ?? 'null'), expected.entries);
		assert.deepEqual(JSON.parse(utility?.content ?? 'null'), expected.utility);
		assert.deepEqual(JSON.parse(matcher?.content ?? 'null'), expected.matcher);
		assert.ok(file.report.sources.every((source) => source.origin.kind === 'file'));
		assert.equal(inline.report.limits.max_file_bytes, 1_048_576n);
		assert.equal(typeof inline.report.findings[0]?.line, 'bigint');
	} finally {
		await backend.close();
		emptyBase.remove();
		project.remove();
	}
});

/** The normalized sources a valid check admits, parsed, in report order. */
async function admittedSources(configuration: Configuration): Promise<Record<string, unknown>> {
	const outcome = await check(configuration);
	assert.equal(outcome.kind, 'valid');
	if (outcome.kind !== 'valid') return {};
	const [root, , entries, utility, matcher] = outcome.sources.map((source): unknown =>
		source.representation === 'normalized_value' ? JSON.parse(source.content) : source.content,
	);
	return { root, entries, utility, matcher };
}

await test('the full source comparison catches a changed setting, metadata value or default member', async () => {
	const base = ownedDirectory('source-controls');
	try {
		const endpoint = 'http://127.0.0.1:1/v1/systemone';
		const original = inlineConfiguration(base.path, endpoint);
		const expected: Record<string, unknown> = expectedInlineSources(endpoint);
		assert.deepEqual(await admittedSources(original), expected);
		const evidence = original.evidence.kind === 'inline' ? original.evidence : undefined;
		const paragraph = evidence?.matchers['paragraph'];
		assert.ok(evidence !== undefined && paragraph !== undefined);
		const variants: readonly (readonly [string, Configuration])[] = [
			['root', { ...original, limits: { max_file_bytes: 1_048_576n, max_files: 7 } }],
			[
				'root',
				{ ...original, check: { files: ['skills/**/SKILL.md'], ignore: ['skills/beta/**'] } },
			],
			[
				'matcher',
				{
					...original,
					evidence: {
						...evidence,
						matchers: { paragraph: { ...paragraph, metadata: { owner: 'docs', weight: 3 } } },
					},
				},
			],
			[
				'matcher',
				{
					...original,
					evidence: {
						...evidence,
						matchers: { paragraph: { ...paragraph, message: 'Paragraph.' } },
					},
				},
			],
		];
		const admittedVariants = await Promise.all(
			variants.map(
				async ([changed, variant]) => [changed, await admittedSources(variant)] as const,
			),
		);
		for (const [changed, admitted] of admittedVariants) {
			// Only the changed source differs, and the comparison sees it.
			assert.throws(() => assert.deepStrictEqual(admitted[changed], expected[changed]));
			for (const name of Object.keys(expected).filter((key) => key !== changed)) {
				assert.deepEqual(admitted[name], expected[name], `${name} unchanged`);
			}
		}
	} finally {
		base.remove();
	}
});

const { only_when: _gate, ...ungatedRule } = ruleEntries.K001;

function askingConfiguration(id: string, base: string, endpoint: string): Configuration {
	return {
		id,
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['notes/*.md'] },
		decision_model: { model: 'jev-latest', endpoint },
		evidence: {
			kind: 'inline',
			utilities: {},
			matchers: { paragraph: { language: 'Markdown', rule: { kind: 'paragraph' } } },
			kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
		},
		rulebooks: [
			{
				kind: 'inline',
				id: `${id}-rules`,
				markdown: rulebookMarkdown,
				entries: { K001: ungatedRule },
			},
		],
	} satisfies Configuration;
}

/** Whether a captured request came from the overlap test's run A. */
function fromA(request: CapturedRequest): boolean {
	return JSON.stringify(request.body).includes('run-a-marker');
}

async function requestFrom(
	backendUnderTest: ControlledBackend,
	marker: string,
	pending: CapturedRequest[],
): Promise<CapturedRequest> {
	const held = pending.find((candidate) => JSON.stringify(candidate.body).includes(marker));
	if (held !== undefined) return held;
	pending.push(await backendUnderTest.nextRequest());
	return requestFrom(backendUnderTest, marker, pending);
}

await test('aborting one of two overlapping runs cancels only that run and settles its work', async () => {
	const base = ownedDirectory('overlap-base');
	const rootA = ownedDirectory('overlap-a');
	const rootB = ownedDirectory('overlap-b');
	const gated = await startControlledBackend();
	try {
		writeFiles(rootA.path, {
			'notes/a.md': 'First paragraph from run-a-marker.\n\nSecond paragraph from run-a-marker.\n',
		});
		writeFiles(rootB.path, { 'notes/b.md': 'Only paragraph from run-b-marker.\n' });
		const abortA = new AbortController();
		const runA = run(askingConfiguration('run-a', base.path, gated.endpoint), {
			root: rootA.path,
			scope: { kind: 'all' },
			signal: abortA.signal,
		});
		const runB = run(askingConfiguration('run-b', base.path, gated.endpoint), {
			root: rootB.path,
			scope: { kind: 'all' },
		});

		// Causal gate: both runs have a request in flight, neither answered. A asks about
		// each of its two paragraphs at once; wait for both, since the server records a
		// request only after reading its whole body, so A's set is complete before the abort.
		const pending: CapturedRequest[] = [];
		const heldA = await requestFrom(gated, 'run-a-marker', pending);
		const heldB = await requestFrom(gated, 'run-b-marker', pending);
		await waitUntil(
			() => gated.received.filter(fromA).length === 2,
			'both run-A paragraph requests received before the abort',
		);
		abortA.abort();
		const outcomeA = await runA;
		assert.equal(outcomeA.kind, 'cancelled', describe(outcomeA));
		if (outcomeA.kind === 'cancelled') assert.equal(outcomeA.report?.status.kind, 'cancelled');
		// Every request A sent must be closed by A's client. The server learns of a close
		// on a later event-loop turn, so wait for that event, bounded; an A request that
		// stays open (an uncancelled native request) times out and fails.
		const requestsFromA = gated.received.filter(fromA);
		assert.ok(requestsFromA.includes(heldA));
		await waitUntil(
			() => requestsFromA.every((request) => request.closedByClient()),
			`all ${requestsFromA.length} run-A requests closed by the client`,
		);
		assert.ok(!heldB.closedByClient(), 'B is untouched');

		answerNoul(heldB, 0.1);
		for (const other of pending.filter((candidate) => candidate !== heldB && !fromA(candidate))) {
			answerNoul(other, 0.1);
		}
		const outcomeB = await runB;
		assert.equal(outcomeB.kind, 'completed', describe(outcomeB));
		if (outcomeB.kind === 'completed') {
			assert.equal(outcomeB.report.counts.warn, 1);
			assert.deepEqual(outcomeB.report.configuration, {
				kind: 'inline',
				id: 'run-b',
				label: 'run-b',
			});
			assert.ok(outcomeB.report.findings.every((finding) => finding.file === 'notes/b.md'));
		}
		// While B ran to completion after A returned, A sent nothing more.
		assert.deepEqual(gated.received.filter(fromA), requestsFromA);
	} finally {
		await gated.close();
		base.remove();
		rootA.remove();
		rootB.remove();
	}
});

async function waitUntil(condition: () => boolean, description: string): Promise<void> {
	const deadline = Date.now() + 3000;
	while (!condition()) {
		if (Date.now() > deadline) assert.fail(`timed out waiting: ${description}`);
		// oxlint-disable-next-line no-await-in-loop -- polling one observed socket state
		await new Promise((resolve) => setTimeout(resolve, 10));
	}
}

function countingBinding(transform: (binding: NativeBinding) => NativeBinding): {
	readonly calls: { run: number };
	readonly sdk: ReturnType<typeof createSdkCalls>;
} {
	const calls = { run: 0 };
	const sdk = createSdkCalls(() => {
		const resolved = resolveBinding(installedBindingSources());
		if (resolved.kind === 'failed') return resolved;
		const changed = transform(resolved.binding);
		return {
			...resolved,
			binding: {
				...changed,
				contractInfo: () => changed.contractInfo(),
				CallCancellation: changed.CallCancellation,
				load: (options) => changed.load(options),
				check: (configuration, cancellation) => changed.check(configuration, cancellation),
				run: (configuration, request, cancellation) => {
					calls.run += 1;
					return changed.run(configuration, request, cancellation);
				},
			},
		};
	});
	return { calls, sdk };
}

await test('an already-aborted signal starts no native call and sends no request', async () => {
	const base = ownedDirectory('pre-abort');
	try {
		const { calls, sdk } = countingBinding((binding) => binding);
		const receivedBefore = sharedBackend.received.length;
		const aborted = AbortSignal.abort();
		// Even a value that would fail encoding is reported as cancelled.
		const unencodable = {
			...askingConfiguration('pre', base.path, sharedBackend.endpoint),
			limits: { max_files: Number.NaN },
		};
		const outcome = await sdk.run(unencodable, {
			root: repository.path,
			scope: { kind: 'all' },
			signal: aborted,
		});
		assert.deepEqual(outcome, { kind: 'cancelled', report: null });
		assert.equal(calls.run, 0);
		assert.equal(sharedBackend.received.length, receivedBefore);
	} finally {
		base.remove();
	}
});

await test('an unreadable or failed native reply after dispatch is failed with unknown effects, never retried', async () => {
	const base = ownedDirectory('reply');
	const backend = await startControlledBackend();
	answerSkillRequests(backend);
	try {
		const garbled = countingBinding((binding) => ({
			...binding,
			run: async (configuration, request, cancellation) => {
				await binding.run(configuration, request, cancellation);
				return '{"kind":"completed","report":{"status":{"kind":"prepared"}}}';
			},
		}));
		const receivedBefore = backend.received.length;
		const outcome = await garbled.sdk.run(inlineConfiguration(base.path, backend.endpoint), {
			root: repository.path,
			scope: { kind: 'all' },
		});
		assert.equal(outcome.kind, 'failed');
		if (outcome.kind === 'failed') {
			assert.deepEqual(
				[outcome.failure.phase, outcome.failure.effects, outcome.failure.reason],
				['after_dispatch', 'unknown', 'native_reply_invalid'],
			);
			assert.equal(outcome.report, null);
		}
		assert.equal(garbled.calls.run, 1, 'one native invocation, no retry');
		assert.ok(
			backend.received.length > receivedBefore,
			'effects did happen before the unreadable reply',
		);

		const rejected = countingBinding((binding) => ({
			...binding,
			run: async () => Promise.reject(new Error('native task stopped')),
		}));
		const failed = await rejected.sdk.run(inlineConfiguration(base.path, backend.endpoint), {
			root: repository.path,
			scope: { kind: 'all' },
		});
		assert.equal(
			failed.kind === 'failed' ? failed.failure.reason : failed.kind,
			'native_call_failed',
		);
		assert.equal(rejected.calls.run, 1);
	} finally {
		await backend.close();
		base.remove();
	}
});

await sharedBackend.close();
repository.remove();
