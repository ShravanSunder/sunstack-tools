// Model-backed runs through the real addon and Rust executor. Only model answers
// come from the controlled loopback backend; this is not hosted-provider proof.
import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Configuration, RunOutcome } from '../src/index.ts';
import { load, run } from '../src/index.ts';
import type { NativeBinding } from '../src/native-binding.ts';
import { installedBindingSources, resolveBinding } from '../src/native-binding.ts';
import { createSdkCalls } from '../src/sdk-calls.ts';
import type { CapturedRequest, ControlledBackend } from './support/controlled-backend.ts';
import { answerChoice, answerNoul, startControlledBackend } from './support/controlled-backend.ts';
import type { OwnedDirectory } from './support/sdk-fixtures.ts';
import {
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
		const inline = await run(inlineConfiguration(emptyBase.path, backend.endpoint), {
			root: repository.path,
			scope: { kind: 'all' },
		});
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
		assert.ok(file.report.sources.every((source) => source.origin.kind === 'file'));
		assert.equal(inline.report.limits.max_file_bytes, 1_048_576n);
		assert.equal(typeof inline.report.findings[0]?.line, 'bigint');
	} finally {
		await backend.close();
		emptyBase.remove();
		project.remove();
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

		// Causal gate: both runs have a request in flight, neither answered.
		const pending: CapturedRequest[] = [];
		const heldA = await requestFrom(gated, 'run-a-marker', pending);
		const heldB = await requestFrom(gated, 'run-b-marker', pending);
		abortA.abort();
		const outcomeA = await runA;
		assert.equal(outcomeA.kind, 'cancelled', describe(outcomeA));
		if (outcomeA.kind === 'cancelled') assert.equal(outcomeA.report?.status.kind, 'cancelled');
		await waitUntil(() => heldA.closedByClient(), 'A closed its in-flight request');
		assert.ok(!heldB.closedByClient(), 'B is untouched');

		answerNoul(heldB, 0.1);
		for (const other of pending.filter((candidate) => candidate !== heldA && candidate !== heldB)) {
			assert.ok(!JSON.stringify(other.body).includes('run-a-marker') || other.closedByClient());
			if (!other.closedByClient()) answerNoul(other, 0.1);
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
		// Nothing from A arrives after A returned.
		const afterA = gated.received.length;
		await new Promise((resolve) => setTimeout(resolve, 200));
		assert.ok(
			gated.received
				.slice(afterA)
				.every((request) => !JSON.stringify(request.body).includes('run-a-marker')),
		);
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
