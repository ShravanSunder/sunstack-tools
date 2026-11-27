// Real addon and HTTP transport; the loopback backend replaces model answers only.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { CONTRACT_REVISION } from '../src/contracts/contract-revision.ts';
import type {
	CaseBinding,
	CaseExecutionOwner,
	Configuration,
	SdkDocumentValue,
	SdkReport,
} from '../src/index.ts';
import { deserialize, run, serialize } from '../src/index.ts';
import { isRunOutcome } from '../src/reply-decoding.ts';
import type { CapturedRequest } from './support/controlled-backend.ts';
import { withAnsweringBackend } from './support/controlled-backend.ts';
import { writeFiles } from './support/sdk-fixtures.ts';

process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

// Independent source oracle: both elements occupy line 1 and each is 12 UTF-8 bytes.
const page = '<p>café</p><p>βeta</p>\r\n';
const firstBinding = {
	id: 'first',
	kind: 'rule',
	rule: 'clear',
	file: 'docs/page.html',
	lines: [1n, 1n],
	bytes: [0n, 12n],
	text_sha256: createHash('sha256').update('<p>café</p>').digest('hex'),
} as const satisfies CaseBinding;
const secondBinding = {
	...firstBinding,
	id: 'second',
	bytes: [12n, 24n],
	text_sha256: createHash('sha256').update('<p>βeta</p>').digest('hex'),
} as const satisfies CaseBinding;
const firstOwner = {
	kind: 'rule',
	rule: 'clear',
	file: 'docs/page.html',
	lines: [1n, 1n],
	bytes: [0n, 12n],
} as const satisfies CaseExecutionOwner;
const secondOwner = {
	...firstOwner,
	bytes: [12n, 24n],
} as const satisfies CaseExecutionOwner;

function configuration(root: string, endpoint: string, selection: boolean = false): Configuration {
	return {
		id: 'case-byte-binding',
		base_directory: root,
		marker_prefix: 'K',
		check: { files: ['docs/*.html'] },
		decision_model: { model: 'jev-latest', endpoint },
		evidence: {
			kind: 'inline',
			utilities: {},
			matchers: { paragraph: { language: 'Html', rule: { kind: 'element', regex: '^<p>' } } },
			kinds: {
				paragraph: {
					rule: 'paragraph',
					fields: { body: { from: 'remainder' } },
					...(selection
						? {
								select: {
									question: {
										type: 'noul' as const,
										instructions: { check: 'body', task: 'Keep this paragraph?' },
									},
									yes_at_or_above: 0.8,
									no_at_or_below: 0.2,
								},
							}
						: {}),
				},
			},
		},
		rulebooks: [
			{
				kind: 'inline',
				id: 'case-rules',
				markdown: 'Each paragraph is clear. <!-- K001 -->\n',
				entries: {
					K001: {
						name: 'clear',
						evidence: 'paragraph',
						severity: 'error',
						questions: {
							clear: { type: 'noul', instructions: { check: 'body', task: 'Is it clear?' } },
						},
						decision: {
							start: 'clear',
							steps: {
								clear: {
									ask: 'clear',
									yes_at_or_above: 0.8,
									no_at_or_below: 0.2,
									yes: 'pass',
									no: { finding: 'Unclear.' },
									uncertain: 'uncertain',
								},
							},
						},
					},
				},
			},
		],
	};
}

function wireFor(
	report: SdkReport,
	requests: readonly CapturedRequest[],
	start: bigint,
): CapturedRequest {
	const saved = report.requests.filter(
		(request) => request.owner.kind === 'rule' && request.owner.piece.span.byte_start === start,
	);
	assert.equal(saved.length, 1);
	const request = saved[0];
	assert.ok(request !== undefined);
	const matching = requests.filter((wire) => {
		try {
			assert.deepEqual(wire.body['state'], request.request.state);
			assert.deepEqual(wire.body['questions'], request.request.questions);
			return true;
		} catch {
			return false;
		}
	});
	assert.equal(matching.length, 1);
	const wire = matching[0];
	assert.ok(wire !== undefined);
	return wire;
}

async function roundTrip(payload: SdkDocumentValue): Promise<SdkDocumentValue> {
	const saved = serialize(payload);
	assert.equal(saved.kind, 'serialized');
	if (saved.kind !== 'serialized') throw new Error('byte report serialization failed');
	const restored = await deserialize(saved.json);
	assert.equal(restored.kind, 'decoded');
	if (restored.kind !== 'decoded') throw new Error('byte report decode failed');
	assert.deepEqual(restored.value, payload);
	return restored.value;
}

await test('same-line HTML byte bindings select independently, retain exact owners and deduplicate execution', async () => {
	const { answered } = await withAnsweringBackend(
		'case-bytes',
		4,
		async (backend, root): Promise<void> => {
			writeFiles(root, { 'docs/page.html': page });
			const config = configuration(root, backend.endpoint);
			const options = { root, scope: { kind: 'all' } } as const;
			const ordinary = await run(config, options);
			assert.equal(ordinary.kind, 'completed');
			if (ordinary.kind !== 'completed') throw new Error('ordinary byte control failed');
			assert.equal(ordinary.report.results.length, 2);
			assert.equal(backend.received.length, 2);
			const originalWires = [...backend.received];
			assert.deepEqual(
				Object.values(ordinary.report.piece_evidence)
					.map(({ piece }) => [piece.where.span.byte_start, piece.where.span.byte_end])
					.toSorted((left, right) => Number((left[0] ?? 0n) - (right[0] ?? 0n))),
				[
					[0n, 12n],
					[12n, 24n],
				],
			);
			const selected = await run(config, {
				...options,
				case_selection: [firstBinding, { ...firstBinding, id: 'same-owner' }],
			});
			assert.equal(selected.kind, 'completed');
			if (selected.kind !== 'completed') throw new Error('first byte selection failed');
			assert.equal(backend.received.length, 3, 'two IDs for one owner send one request');
			assert.equal(selected.report.results.length, 1);
			assert.equal(selected.report.requests.length, 1);
			const selectedRequestOwner = selected.report.requests[0]?.owner;
			assert.ok(selectedRequestOwner?.kind === 'rule');
			assert.deepEqual(
				[selectedRequestOwner.piece.span.byte_start, selectedRequestOwner.piece.span.byte_end],
				[0n, 12n],
			);
			const selectedSource = selectedRequestOwner.piece.span.source;
			assert.ok(selectedSource.kind === 'document');
			assert.equal(
				selectedSource.content_identity,
				createHash('sha256').update(page).digest('hex'),
			);

			assert.deepEqual(
				selected.report.case_selection?.bindings.map((item) => item.outcome),
				[
					{ kind: 'selected', owner: firstOwner },
					{ kind: 'selected', owner: firstOwner },
				],
			);
			assert.deepEqual(selected.report.case_selection?.excluded, [secondOwner]);
			assert.deepEqual(
				selected.report.case_selection?.bindings.map((item) => item.binding),
				[firstBinding, { ...firstBinding, id: 'same-owner' }],
			);
			assert.deepEqual(
				wireFor(selected.report, backend.received.slice(2), 0n).rawBody,
				wireFor(ordinary.report, originalWires, 0n).rawBody,
				'selection preserves complete model payload bytes',
			);
			const result = selected.report.results[0];
			assert.ok(result !== undefined);
			assert.equal(selected.report.piece_evidence[String(result.id)]?.piece.text, '<p>café</p>');
			await roundTrip({ kind: 'run_outcome', value: selected });
			await roundTrip({ kind: 'report', value: selected.report });

			const second = await run(config, { ...options, case_selection: [secondBinding] });
			assert.equal(second.kind, 'completed');
			if (second.kind !== 'completed') throw new Error('second byte selection failed');
			assert.equal(second.report.results.length, 1);
			assert.equal(second.report.requests.length, 1);
			const secondRequestOwner = second.report.requests[0]?.owner;
			assert.ok(secondRequestOwner?.kind === 'rule');
			assert.deepEqual(
				[secondRequestOwner.piece.span.byte_start, secondRequestOwner.piece.span.byte_end],
				[12n, 24n],
			);

			assert.deepEqual(second.report.case_selection?.bindings[0]?.outcome, {
				kind: 'selected',
				owner: secondOwner,
			});
			assert.deepEqual(second.report.case_selection?.excluded, [firstOwner]);
			assert.deepEqual(
				wireFor(second.report, backend.received.slice(3), 12n).rawBody,
				wireFor(ordinary.report, originalWires, 12n).rawBody,
			);
		},
	);
	assert.equal(answered, 4);
});

await test('native geometry, captured bounds and stale selection demand settle before model dispatch', async () => {
	const { answered } = await withAnsweringBackend(
		'case-byte-admission',
		2,
		async (backend, root): Promise<void> => {
			writeFiles(root, { 'docs/page.html': page });
			const config = configuration(root, backend.endpoint, true);
			const options = { root, scope: { kind: 'all' } } as const;
			const positive = await run(config, { ...options, case_selection: [firstBinding] });
			assert.equal(positive.kind, 'completed');
			if (positive.kind !== 'completed') throw new Error('selection observer positive failed');
			assert.deepEqual(positive.report.case_selection?.bindings[0]?.outcome, {
				kind: 'selected',
				owner: firstOwner,
			});
			assert.equal(
				backend.received.length,
				2,
				'observer witnessed model selection and one rule request',
			);
			assert.equal(
				positive.report.requests.filter((request) => request.owner.kind === 'selection').length,
				1,
			);
			assert.equal(
				positive.report.requests.filter((request) => request.owner.kind === 'rule').length,
				1,
			);
			const invalidRanges = [
				[12n, 0n],
				[0n, 27n],
			] as const;
			await Promise.all(
				invalidRanges.map(async (bytes): Promise<void> => {
					const invalid = await run(config, {
						...options,
						case_selection: [firstBinding, { ...firstBinding, id: 'invalid', bytes }],
					});
					assert.equal(invalid.kind, 'invalid');
					if (invalid.kind !== 'invalid') throw new Error('invalid byte geometry was accepted');
					assert.ok(
						invalid.diagnostics.some((diagnostic) => diagnostic.code === 'unsupported_inputs'),
					);
					assert.equal(
						backend.received.length,
						2,
						'invalid input sends no selection or rule requests',
					);
				}),
			);
			const unsafe: unknown = await Reflect.apply(run, undefined, [
				config,
				{
					...options,
					case_selection: [
						{ ...firstBinding, bytes: [Number.MAX_SAFE_INTEGER + 2, 9007199254740994n] },
					],
				},
			]);
			assert.ok(isRunOutcome(unsafe));
			assert.equal(unsafe.kind, 'failed');
			if (unsafe.kind !== 'failed') throw new Error('unsafe byte number was accepted');
			assert.deepEqual(
				[unsafe.failure.phase, unsafe.failure.effects, unsafe.failure.reason],
				['before_dispatch', 'none', 'encoding_rejected'],
			);
			assert.equal(backend.received.length, 2);

			const staleBinding = { ...firstBinding, id: 'stale', text_sha256: '0'.repeat(64) };
			const stale = await run(config, { ...options, case_selection: [staleBinding] });
			assert.equal(stale.kind, 'completed');
			if (stale.kind !== 'completed') throw new Error('stale byte run failed');
			assert.deepEqual(stale.report.case_selection?.bindings, [
				{ binding: staleBinding, outcome: { kind: 'stale' } },
			]);
			assert.equal(stale.report.requests.length, 0);
			assert.equal(stale.report.results.length, 0);
			assert.equal(
				backend.received.length,
				2,
				'stale identity creates no selection or rule demand',
			);

			const unacquiredBinding = {
				...firstBinding,
				id: 'unacquired-wide',
				file: 'outside/page.html',
				bytes: [9007199254740993n, 9007199254740994n],
			} as const satisfies CaseBinding;
			const unacquired = await run(config, { ...options, case_selection: [unacquiredBinding] });
			assert.equal(unacquired.kind, 'completed');
			if (unacquired.kind !== 'completed') throw new Error('unacquired binding failed');
			assert.deepEqual(unacquired.report.case_selection?.bindings, [
				{ binding: unacquiredBinding, outcome: { kind: 'unmatched' } },
			]);
			assert.equal(unacquired.report.requests.length, 0);
			assert.equal(backend.received.length, 2);
			await roundTrip({ kind: 'run_outcome', value: unacquired });
		},
	);
	assert.equal(answered, 2);
});

await test('genuine old20 and old21 documents are incompatible before their payload is decoded', async () => {
	const documents = [
		{ revision: 20, sha256: '745352414fdf39fa6614e12173588ac7b868ab24db0e22225b4caca171961ee9' },
		{ revision: 21, sha256: '912407069a2312231c04bcda5adcc234f9df444e6fcbae132b83daf4b2122109' },
	] as const;
	await Promise.all(
		documents.map(async ({ revision, sha256 }): Promise<void> => {
			const original = readFileSync(
				new URL(`./fixtures/codec/revision${revision}-synthetic.sdk-document`, import.meta.url),
				'utf8',
			);
			assert.equal(
				createHash('sha256').update(original).digest('hex'),
				sha256,
				'genuine old writer bytes stay unchanged',
			);
			const prefix = `{"contract_revision":${revision},"payload":`;
			assert.ok(original.startsWith(prefix), 'preserve genuine old producer framing');
			// The second specimen keeps the genuine framing but its payload cannot decode.
			const invalidPayload = `${prefix}{"future":184467440737095516160000}}`;
			await Promise.all(
				[original, invalidPayload].map(async (document): Promise<void> => {
					const restored = await deserialize(document);
					assert.equal(restored.kind, 'incompatible');
					if (restored.kind !== 'incompatible')
						throw new Error('old document reached payload decoder');
					assert.equal(restored.actual_contract_revision, revision);
					assert.equal(restored.expected_contract_revision, CONTRACT_REVISION);
				}),
			);
		}),
	);
});
