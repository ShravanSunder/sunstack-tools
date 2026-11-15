// Real addon and HTTP boundary: selection must remove owners before dispatch.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { test } from 'node:test';

import type { CaseBinding, Configuration, SdkReport } from '../src/index.ts';
import { deserialize, run, serialize } from '../src/index.ts';
import type { CapturedRequest } from './support/controlled-backend.ts';
import { withAnsweringBackend } from './support/controlled-backend.ts';
import { jsonSlice } from './support/raw-json.ts';
import { writeFiles } from './support/sdk-fixtures.ts';

process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';
const selectedLine = 'Keep café instructions.\n';
const binding = {
	id: 'selected',
	kind: 'rule',
	rule: 'clear',
	file: 'docs/one.md',
	lines: [3n, 3n],
	text_sha256: createHash('sha256').update(selectedLine).digest('hex'),
} as const satisfies CaseBinding;
function configuration(root: string, endpoint: string): Configuration {
	return {
		id: 'case-dispatch',
		base_directory: root,
		marker_prefix: 'K',
		check: { files: ['docs/*.md'] },
		decision_model: { model: 'jev-latest', endpoint },
		evidence: {
			kind: 'inline',
			utilities: {},
			matchers: {},
			kinds: {
				paragraph: {
					markdown_units: { mode: 'unit', units: ['paragraph'] },
					fields: { body: { from: 'remainder' } },
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
							clear: {
								type: 'noul',
								instructions: { check: 'body', context: ['where'], task: 'Is it clear?' },
							},
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
/** Match the rule question and exact source owner, independent of request ordinal. */
function isSelectedOwner(request: CapturedRequest): boolean {
	const state = request.body['state'];
	const questions = request.body['questions'];
	if (
		typeof state !== 'object' ||
		state === null ||
		!('where' in state) ||
		typeof questions !== 'object' ||
		questions === null ||
		!('clear' in questions)
	)
		return false;
	const location = state.where;
	return (
		typeof location === 'object' &&
		location !== null &&
		'file' in location &&
		location.file === binding.file &&
		'first_line' in location &&
		location.first_line === 3 &&
		'last_line' in location &&
		location.last_line === 3
	);
}

/** Hash the original SDK report's selected request tokens, excluding requested_model. */
function runtimeInputDigest(report: SdkReport): string {
	const indices = report.requests.flatMap((request, index) =>
		request.owner.kind === 'rule' &&
		request.owner.rule === binding.rule &&
		request.owner.piece.file === binding.file &&
		request.owner.piece.first_line === 3n &&
		request.owner.piece.last_line === 3n
			? [index]
			: [],
	);
	assert.equal(indices.length, 1);
	const index = indices[0];
	assert.ok(index !== undefined);
	const saved = serialize({ kind: 'report', value: report });
	assert.equal(saved.kind, 'serialized');
	if (saved.kind !== 'serialized') throw new Error('report serialization failed');
	const rawInput = jsonSlice(saved.json, ['payload', 'value', 'requests', index, 'request']);
	assert.ok(Buffer.byteLength(rawInput, 'utf8') > 2);
	const digest = createHash('sha256').update(rawInput).digest('hex');
	const nativeDigest = report.requests[index]?.input_sha256;
	if (nativeDigest !== undefined && nativeDigest !== null) assert.equal(nativeDigest, digest);
	return digest;
}

await test('same-file case selection dispatches once, preserves payloads, accounts every binding and round-trips exact reports', async () => {
	const { answered } = await withAnsweringBackend(
		'case-dispatch',
		3,
		async (backend, root): Promise<void> => {
			writeFiles(root, { 'docs/one.md': `# Guide\n\n${selectedLine}\nExcluded β instructions.\n` });
			const config = configuration(root, backend.endpoint);
			const options = { root, scope: { kind: 'all' } } as const;
			const ordinary = await run(config, options);
			assert.equal(ordinary.kind, 'completed');
			if (ordinary.kind !== 'completed') throw new Error('ordinary control failed');
			assert.equal(backend.received.length, 2, 'ordinary observer sees both eligible paragraphs');
			assert.equal(ordinary.report.case_selection, undefined);
			const ordinaryWireRequests = backend.received.filter(isSelectedOwner);
			assert.equal(
				ordinaryWireRequests.length,
				1,
				'one ordinary request has the selected semantic owner',
			);
			const ordinaryWireRequest = ordinaryWireRequests[0];
			assert.ok(ordinaryWireRequest !== undefined);
			const ordinaryRequest = ordinary.report.requests.find(
				(request) => request.owner.kind === 'rule' && request.owner.piece.first_line === 3n,
			);
			assert.ok(ordinaryRequest !== undefined);
			const selected = await run(config, {
				...options,
				case_selection: [
					binding,
					{ ...binding, id: 'same-owner' },
					{ ...binding, id: 'unmatched', rule: 'absent' },
					{ ...binding, id: 'stale', text_sha256: '0'.repeat(64) },
				],
			});
			assert.equal(selected.kind, 'completed');
			if (selected.kind !== 'completed') throw new Error('selected run failed');
			assert.equal(backend.received.length, 3, 'only one selected owner reaches HTTP');
			const selectedWireRequests = backend.received.slice(2).filter(isSelectedOwner);
			assert.equal(
				selectedWireRequests.length,
				1,
				'one selected request has the same semantic owner',
			);
			const selectedWireRequest = selectedWireRequests[0];
			assert.ok(selectedWireRequest !== undefined);
			assert.ok(ordinaryWireRequest.rawBody.byteLength > 0);
			assert.ok(selectedWireRequest.rawBody.byteLength > 0);
			assert.deepEqual(
				selectedWireRequest.rawBody,
				ordinaryWireRequest.rawBody,
				'complete HTTP bytes are identical',
			);
			const ordinaryWireDigest = createHash('sha256')
				.update(ordinaryWireRequest.rawBody)
				.digest('hex');
			const selectedWireDigest = createHash('sha256')
				.update(selectedWireRequest.rawBody)
				.digest('hex');
			assert.match(ordinaryWireDigest, /^[a-f0-9]{64}$/u);
			assert.equal(
				selectedWireDigest,
				ordinaryWireDigest,
				'independent actual-wire digests are identical',
			);
			assert.deepEqual(selectedWireRequest.body, ordinaryWireRequest.body);
			assert.equal(selected.report.requests.length, 1);
			assert.deepEqual(selected.report.requests[0]?.request, ordinaryRequest.request);
			// Input hash excludes requested_model; whole HTTP-byte parity is a separate guard.
			const ordinaryInputDigest = runtimeInputDigest(ordinary.report);
			const selectedInputDigest = runtimeInputDigest(selected.report);
			assert.match(ordinaryInputDigest, /^[a-f0-9]{64}$/u);
			assert.equal(
				selectedInputDigest,
				ordinaryInputDigest,
				'exact serialized request input digests are identical',
			);
			const accounting = selected.report.case_selection;
			assert.ok(accounting !== undefined && accounting !== null);
			assert.deepEqual(
				accounting.bindings.map((result) => [result.binding.id, result.outcome.kind]),
				[
					['selected', 'selected'],
					['same-owner', 'selected'],
					['unmatched', 'unmatched'],
					['stale', 'stale'],
				],
			);
			assert.equal(accounting.excluded.length, 1);
			assert.deepEqual(accounting.bindings[0]?.binding, binding);
			const payloads = [
				{ kind: 'run_outcome', value: selected },
				{ kind: 'report', value: selected.report },
			] as const;
			await Promise.all(
				payloads.map(async (payload): Promise<void> => {
					const saved = serialize(payload);
					assert.equal(saved.kind, 'serialized');
					if (saved.kind !== 'serialized') throw new Error('report serialization failed');
					const restored = await deserialize(saved.json);
					assert.equal(restored.kind, 'decoded');
					if (restored.kind !== 'decoded') throw new Error('codec failed');
					assert.deepEqual(restored.value, payload);
					// Wide binding lines must retain adjacent exact values, even in a saved report.
					const wide = saved.json.replace(
						'"lines":[3,3]',
						'"lines":[9007199254740993,9007199254740994]',
					);
					assert.notEqual(wide, saved.json);
					const wideRead = await deserialize(wide);
					assert.equal(wideRead.kind, 'decoded');
					if (wideRead.kind !== 'decoded') throw new Error('wide report decode failed');
					const value = wideRead.value;
					const report =
						value.kind === 'report'
							? value.value
							: value.kind === 'run_outcome' && value.value.kind === 'completed'
								? value.value.report
								: undefined;
					const subject = report?.case_selection?.bindings[0]?.binding;
					assert.ok(subject?.kind === 'rule');
					assert.deepEqual(subject.lines, [9007199254740993n, 9007199254740994n]);
					const overflow = await deserialize(
						wide.replace('9007199254740993', '18446744073709551616'),
					);
					assert.equal(overflow.kind, 'invalid');
					const framed: unknown = JSON.parse(saved.json);
					assert.ok(
						typeof framed === 'object' &&
							framed !== null &&
							'contract_revision' in framed &&
							typeof framed.contract_revision === 'number',
					);
					const mismatched = await deserialize(
						saved.json.replace(
							`"contract_revision":${framed.contract_revision}`,
							`"contract_revision":${framed.contract_revision - 1}`,
						),
					);
					assert.equal(mismatched.kind, 'incompatible');
				}),
			);
			const wideInput = {
				...binding,
				id: 'wide-input',
				lines: [9007199254740993n, 9007199254740994n],
			} as const satisfies CaseBinding;
			const wideRun = await run(config, { ...options, case_selection: [wideInput] });
			assert.equal(wideRun.kind, 'completed');
			if (wideRun.kind !== 'completed') throw new Error('wide input transport failed');
			assert.deepEqual(wideRun.report.case_selection?.bindings[0]?.binding, wideInput);
			assert.equal(wideRun.report.case_selection?.bindings[0]?.outcome.kind, 'stale');
			const duplicateIds = await run(config, { ...options, case_selection: [binding, binding] });
			assert.equal(duplicateIds.kind, 'invalid', 'duplicate ids are diagnosed before dispatch');
			const empty = await run(config, { ...options, case_selection: [] });
			assert.equal(empty.kind, 'completed');
			if (empty.kind !== 'completed') throw new Error('empty run failed');
			assert.equal(empty.report.requests.length, 0);
			assert.deepEqual(empty.report.case_selection?.bindings, []);
			assert.equal(backend.received.length, 3, 'explicit empty selection sends nothing');
		},
	);
	assert.equal(answered, 3);
});
