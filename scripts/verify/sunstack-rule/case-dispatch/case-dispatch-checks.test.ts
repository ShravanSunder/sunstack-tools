import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { describe, expect, test } from 'vitest';

import { indexJson, jsonSlice } from '../prepare/raw-json.ts';
import {
	attributeControlledCaptures,
	indexRequestReport,
	requireCaseAccounting,
	requireControlledDispatch,
	requireManifestSelection,
	requirePreparedRuntimeParity,
	requireSelectedRequestParity,
	requireUniqueOwnerBindings,
	requestOwnerAt,
} from './case-dispatch-checks.ts';
import { readCorpusSelections, scoreSelectionArguments } from './case-dispatch-corpus.ts';
import { createCaseDispatchFixture } from './case-dispatch-fixture.ts';
import { candidateLines } from './case-dispatch-pending.ts';
import { ordinarySnapshotArguments } from './snapshot-arguments.ts';
import { requireSnapshotParity } from './snapshot-parity.ts';

function requestRecord(rule: string, line: number, rawRequest: string): string {
	const digest = createHash('sha256').update(rawRequest).digest('hex');
	return `{"id":${line},"owner":{"kind":"rule","rule":"${rule}","piece":{"file":"docs/a.md","first_line":${line},"last_line":${line}}},"state":"prepared","status":"prepared","input_sha256":"${digest}","request":${rawRequest}}`;
}
function report(...records: string[]): string {
	return `{"mode":"prepare","requests":[${records.join(',')}]}`;
}
function selectedReport(...records: string[]): string {
	return report(...records).replace(
		'"requests":',
		'"case_selection":{"bindings":[{"binding":{"id":"chosen-a"},"outcome":{"kind":"selected"}},{"binding":{"id":"chosen-b"},"outcome":{"kind":"selected"}}],"excluded":[]},"requests":',
	);
}
function http(request: string): string {
	return `{"model":"test/model",${request.slice(1)}`;
}
const chosen = JSON.stringify(['rule', 'same', 'docs/a.md', '1', '1']);
const omitted = JSON.stringify(['rule', 'same', 'docs/a.md', '2', '2']);
const selectedBindings = JSON.stringify([
	{
		id: 'chosen-a',
		kind: 'rule',
		rule: 'same',
		file: 'docs/a.md',
		lines: [1, 1],
		text_sha256: '0'.repeat(64),
	},
	{
		id: 'chosen-b',
		kind: 'rule',
		rule: 'same',
		file: 'docs/a.md',
		lines: [1, 1],
		text_sha256: '0'.repeat(64),
	},
]);
const selectedRequest =
	'{"state":{"text":"café","wide":18446744073709551615},"questions":{"a":{"type":"noul"}}}';
const excludedRequest = '{"state":{"text":"exclude"},"questions":{"a":{"type":"noul"}}}';

describe('selected request parity observation', () => {
	test('indexed JSON slices preserve original tokens and existing duplicate refusals', () => {
		const raw =
			'{ "wide": 9007199254740993, "nested": { "text": "café\\n\\\"quoted\\\"", "order": [1, 2] } }';
		const indexed = indexJson(raw);
		for (const selected of [['wide'], ['nested', 'text'], ['nested', 'order', 1]] as const)
			expect(indexed.slice(selected)).toBe(jsonSlice(raw, selected));
		expect(indexed.slice(['wide'])).toBe('9007199254740993');
		expect(() => indexJson('{"duplicate":1,"duplicate":2}')).toThrow('Repeated JSON path');
		expect(() => indexed.slice(['missing'])).toThrow('Missing JSON path');
	});
	test('snapshot wrapper preserves receipts and refuses changed selected bytes', () => {
		const root = filesystem.mkdtempSync(path.join(tmpdir(), '2026-11-snapshot-probe-'));
		const snapshot = path.join(root, 'snapshot');
		const evidence = path.join(root, 'evidence');
		filesystem.mkdirSync(snapshot);
		filesystem.mkdirSync(evidence);
		const fake = path.join(root, 'fake-cli');
		const baseline = report(
			requestRecord('same', 1, selectedRequest),
			requestRecord('same', 2, excludedRequest),
		);
		const matching = selectedReport(requestRecord('same', 1, selectedRequest));
		const changed = selectedReport(
			requestRecord('same', 1, selectedRequest.replace('café', 'altered')),
		);
		const program = `#!${process.execPath}\nimport fs from 'node:fs';\nconst args=process.argv.slice(2);\nconst target=args[args.indexOf('--prepare')+1];\nconst selected=args.includes('--case-selection');\nconst report=selected?(process.env.CASE_DISPATCH_TEST_MUTATE?${JSON.stringify(changed)}:${JSON.stringify(matching)}):${JSON.stringify(baseline)};\nfs.writeFileSync(target,report);process.stdout.write(report);process.exitCode=6;\n`;
		filesystem.writeFileSync(fake, program, { mode: 0o700 });
		const digest = createHash('sha256').update(filesystem.readFileSync(fake)).digest('hex');
		const wrapper = path.join(
			process.cwd(),
			'scripts/verify/sunstack-rule/case-dispatch/snapshot-observer.ts',
		);
		filesystem.writeFileSync(path.join(root, 'selection.json'), selectedBindings);
		const args = [
			wrapper,
			'lint',
			'--config',
			path.join(snapshot, 'config'),
			'--case-selection',
			path.join(root, 'selection.json'),
			'--prepare',
			path.join(snapshot, 'prepared.json'),
			'--',
			'docs/a.md',
		];
		const environment = {
			PATH: process.env['PATH'],
			CASE_DISPATCH_REAL_CLI: fake,
			CASE_DISPATCH_EVIDENCE: evidence,
			CASE_DISPATCH_EXPECTED_SHA256: digest,
			SUNSTACK_CREDENTIAL_STORE: 'off',
		};
		const success = spawnSync(process.execPath, args, {
			cwd: snapshot,
			env: environment,
			encoding: 'utf8',
		});
		expect(success.status).toBe(6);
		expect(success.stdout).toBe(matching);
		const first = filesystem.readdirSync(evidence)[0];
		expect(first).toBeDefined();
		if (!first) throw new Error('Missing successful snapshot receipt');
		expect(
			JSON.parse(filesystem.readFileSync(path.join(evidence, first, 'parity.json'), 'utf8')),
		).toMatchObject({ selectedExecutionOwners: 1, ordinaryExecutionOwners: 2 });
		const failure = spawnSync(process.execPath, args, {
			cwd: snapshot,
			env: { ...environment, CASE_DISPATCH_TEST_MUTATE: '1' },
			encoding: 'utf8',
		});
		expect(failure.status).not.toBe(6);
		expect(failure.stdout).toBe('');
		const entries = filesystem.readdirSync(evidence);
		expect(entries).toHaveLength(2);
		const failed = entries.find((entry) => entry !== first);
		expect(failed).toBeDefined();
		if (!failed) throw new Error('Missing failed snapshot receipt');
		expect(filesystem.existsSync(path.join(evidence, failed, 'selected.json'))).toBe(true);
		expect(filesystem.existsSync(path.join(evidence, failed, 'parity.json'))).toBe(false);
	});
	test('snapshot wrapper removes only the selection input and preserves named scope', () => {
		const input = [
			'lint',
			'--config',
			'/snapshot/config.json',
			'--format',
			'json',
			'--case-selection',
			'/snapshot/cases.json',
			'--prepare',
			'/snapshot/output.json',
			'--',
			'docs/a.md',
		];
		expect(ordinarySnapshotArguments(input, '/evidence/ordinary.json')).toEqual([
			'lint',
			'--config',
			'/snapshot/config.json',
			'--format',
			'json',
			'--prepare',
			'/evidence/ordinary.json',
			'--',
			'docs/a.md',
		]);
		expect(() =>
			ordinarySnapshotArguments(
				input.filter((value) => value !== '--case-selection'),
				'/evidence/out',
			),
		).toThrow('selection');
		expect(() =>
			ordinarySnapshotArguments(
				[...input.slice(0, 2), '--case-selection', '/another', ...input.slice(2)],
				'/evidence/out',
			),
		).toThrow('selection');
		expect(() => ordinarySnapshotArguments(input, 'relative.json')).toThrow('absolute');
	});
	test('snapshot report comparison requires a nonempty matched owner with exact request and digest', () => {
		const ordinary = report(
			requestRecord('same', 1, selectedRequest),
			requestRecord('same', 2, excludedRequest),
		);
		const selected = selectedReport(requestRecord('same', 1, selectedRequest));
		expect(() =>
			requireSnapshotParity(
				ordinary,
				selected.replace('"id":"chosen-b"', '"id":"lost"'),
				selectedBindings,
			),
		).toThrow('Selected binding denominator differs');
		expect(requireSnapshotParity(ordinary, selected, selectedBindings)).toMatchObject({
			selectedOwners: [chosen],
			selectedExecutionOwners: 1,
		});
		expect(() => requireSnapshotParity(ordinary, selectedReport(), selectedBindings)).toThrow(
			'no selected execution',
		);
		const leaked = selected.replace(
			'"excluded":[]',
			'"excluded":[{"kind":"rule","rule":"same","file":"docs/a.md","lines":[1,1]}]',
		);
		expect(() => requireSnapshotParity(ordinary, leaked, selectedBindings)).toThrow(
			'Excluded execution owner appeared',
		);
		expect(() =>
			requireSnapshotParity(ordinary, selected.replace('café', 'changed'), selectedBindings),
		).toThrow('input hash differs');
		expect(() =>
			requireSnapshotParity(
				report(requestRecord('same', 2, excludedRequest)),
				selected,
				selectedBindings,
			),
		).toThrow('missing in ordinary');
		const extraEligible = selectedReport(
			requestRecord('same', 1, selectedRequest),
			requestRecord('same', 2, excludedRequest),
		);
		expect(() => requireSnapshotParity(ordinary, extraEligible, selectedBindings)).toThrow(
			'not selected by any supplied binding',
		);
		const large = requestRecord('same', 1, selectedRequest)
			.replace('"first_line":1', '"first_line":9007199254740993')
			.replace('"last_line":1', '"last_line":9007199254740993');
		const largeBinding =
			'[{"id":"wide","kind":"rule","rule":"same","file":"docs/a.md","lines":[9007199254740993,9007199254740993],"text_sha256":"digest"}]';
		const largeSelected = selectedReport(large)
			.replace('"id":"chosen-a"', '"id":"wide"')
			.replace(',{"binding":{"id":"chosen-b"},"outcome":{"kind":"selected"}}', '');
		expect(() => requireSnapshotParity(report(large), largeSelected, largeBinding)).not.toThrow();
		expect(() =>
			requireSnapshotParity(
				report(large),
				largeSelected,
				largeBinding.replaceAll('9007199254740993', '9007199254740992'),
			),
		).toThrow('not selected by any supplied binding');
	});
	test('derives only unique complete-line pending candidates, including Unicode', () => {
		expect(candidateLines('α\nstep café\nsecond\nend\n', 'step café\nsecond\n')).toEqual([2, 3]);
		expect(() => candidateLines('prefix step\n', 'step\n')).toThrow('starts inside a line');
		expect(() => candidateLines('step\nstep\n', 'step\n')).toThrow('missing or repeated');
		expect(() => candidateLines('step', 'step')).toThrow('complete final line');
	});
	test('permits only a repository locator change and selects the exact score IDs', () => {
		const root = filesystem.mkdtempSync(path.join(tmpdir(), '2026-11-case-corpus-'));
		const originals = path.join(root, 'originals');
		const copies = path.join(root, 'copies');
		filesystem.mkdirSync(originals);
		filesystem.mkdirSync(copies);
		const manifest = path.join(originals, 'dispatch.json');
		filesystem.writeFileSync(
			manifest,
			JSON.stringify({ files: { 'one.json': { score: ['chosen'], held: ['held'] } } }),
		);
		const cases = '"cases":[{"id":"chosen","line":9007199254740993},{"id":"held"}],"bars":[]';
		filesystem.writeFileSync(path.join(originals, 'one.json'), `{"repository":"old",${cases}}`);
		const copy = path.join(copies, 'one.json');
		filesystem.writeFileSync(copy, `{"repository":${JSON.stringify(root)},${cases}}`);
		const selected = readCorpusSelections(manifest, copies);
		expect(selected).toHaveLength(1);
		const first = selected[0];
		expect(first).toBeDefined();
		if (!first) throw new Error('Missing corpus selection');
		expect(scoreSelectionArguments(first)).toEqual(['--set', 'development', '--case', 'chosen']);
		filesystem.writeFileSync(
			copy,
			`{"repository":${JSON.stringify(root)},${cases.replace('9007199254740993', '9007199254740992')}}`,
		);
		expect(() => readCorpusSelections(manifest, copies)).toThrow('Execution copy changed cases');
	});
	test('authors two independently hashed owners in one historical file', () => {
		const base = filesystem.mkdtempSync(path.join(tmpdir(), '2026-11-case-dispatch-fixture-'));
		const root = path.join(base, 'repository');
		const fixture = createCaseDispatchFixture(root, 'http://127.0.0.1:9/systemone');
		const input: unknown = JSON.parse(filesystem.readFileSync(fixture.casesPath, 'utf8'));
		expect(input).toMatchObject({
			repository: root,
			cases: [
				{
					id: 'selected-a',
					file: 'docs/pieces.md',
					revision: fixture.revision,
					lines: [1, 1],
					text_sha256: createHash('sha256').update('Approved selected café.\n').digest('hex'),
				},
				{
					id: 'selected-b',
					file: 'docs/pieces.md',
					revision: fixture.revision,
					lines: [1, 1],
					text_sha256: createHash('sha256').update('Approved selected café.\n').digest('hex'),
				},
				{
					id: 'excluded',
					file: 'docs/pieces.md',
					revision: fixture.revision,
					lines: [3, 3],
					text_sha256: createHash('sha256').update('Approved excluded sentinel.\n').digest('hex'),
				},
			],
		});
	});
	test('keeps two case IDs while dispatching their shared owner exactly once', () => {
		const expected = new Map([
			['a', chosen],
			['b', chosen],
		]);
		const actual = [
			{ id: 'a', owner: chosen },
			{ id: 'b', owner: chosen },
		];
		const options = { expected, actual, dispatchedOwners: [chosen] };
		expect(() => requireUniqueOwnerBindings(options)).not.toThrow();
		expect(() => requireUniqueOwnerBindings({ ...options, actual: actual.slice(0, 1) })).toThrow(
			'Matched case IDs',
		);
		expect(() =>
			requireUniqueOwnerBindings({ ...options, dispatchedOwners: [chosen, chosen] }),
		).toThrow('dispatched more than once');
		expect(() =>
			requireUniqueOwnerBindings({
				...options,
				actual: [
					{ id: 'a', owner: chosen },
					{ id: 'b', owner: omitted },
				],
			}),
		).toThrow('Matched owner differs');
	});
	test('binds captures only when their saved owner is unambiguous', () => {
		const first = requestRecord('same', 1, selectedRequest).replace(
			'"state":"prepared"',
			'"requested_model":"test/model","state":"prepared"',
		);
		const second = requestRecord('same', 2, excludedRequest).replace(
			'"state":"prepared"',
			'"requested_model":"test/model","state":"prepared"',
		);
		expect(
			attributeControlledCaptures(report(first, second), [
				http(excludedRequest),
				http(selectedRequest),
			]).map((entry) => entry.owner),
		).toEqual([omitted, chosen]);
		expect(() =>
			attributeControlledCaptures(report(first, first), [
				http(selectedRequest),
				http(selectedRequest),
			]),
		).toThrow('ambiguous');
		expect(() =>
			attributeControlledCaptures(report(first), [
				http(selectedRequest).replace('test/model', 'other'),
			]),
		).toThrow('missing or ambiguous');
	});
	test('keeps score IDs distinct from held, pending and unavailable cases', () => {
		const input = {
			scoreIds: ['a', 'b'],
			heldIds: ['held'],
			pendingIds: ['pending'],
			unmatchedIds: ['unmatched'],
			notCollectibleIds: ['not-collectible'],
			caseIds: ['a', 'b', 'held', 'pending', 'unmatched'],
		};
		expect(() => requireManifestSelection(input)).not.toThrow();
		expect(() => requireManifestSelection({ ...input, scoreIds: ['a', 'held'] })).toThrow(
			'overlapping',
		);
		expect(() => requireManifestSelection({ ...input, scoreIds: ['a', 'missing'] })).toThrow(
			'absent',
		);
		expect(() => requireManifestSelection({ ...input, caseIds: [...input.caseIds, 'a'] })).toThrow(
			'Duplicate case-file',
		);
	});
	test('does not merge distinct owner lines above the safe integer boundary', () => {
		const first = requestRecord('same', 1, selectedRequest)
			.replace('"first_line":1', '"first_line":9007199254740992')
			.replace('"last_line":1', '"last_line":9007199254740992');
		const second = first.replaceAll('9007199254740992', '9007199254740993');
		const raw = report(first, second);
		expect(requestOwnerAt(raw, 0)).not.toEqual(requestOwnerAt(raw, 1));
	});
	test('requires an actual neighbor and excludes its attributed HTTP request', () => {
		const ordinary = [
			{ owner: chosen, raw: selectedRequest },
			{ owner: omitted, raw: excludedRequest },
		];
		const options = {
			ordinary,
			selected: [ordinary[0] ?? { owner: '', raw: '{}' }],
			selectedOwners: [chosen],
			excludedOwners: [omitted],
			prerequisiteOwners: [],
		};
		expect(() => requireControlledDispatch(options)).not.toThrow();
		expect(() => requireControlledDispatch({ ...options, selected: ordinary })).toThrow(
			'Excluded owner dispatched HTTP',
		);
		expect(() => requireControlledDispatch({ ...options, ordinary: options.selected })).toThrow(
			'ordinary HTTP witness',
		);
		expect(() =>
			requireControlledDispatch({
				...options,
				selected: [{ owner: chosen, raw: selectedRequest.replace('café', 'other') }],
			}),
		).toThrow('full HTTP body differs');
		expect(() =>
			requireControlledDispatch({
				...options,
				selected: [{ owner: chosen, raw: selectedRequest.replace('"state":', '"state": ') }],
			}),
		).toThrow('full HTTP body differs');
		expect(() => requireControlledDispatch({ ...options, selected: [] })).toThrow('inventory');
	});
	test('accounts a shared selection prerequisite without treating it as an execution owner', () => {
		const prerequisite = JSON.stringify(['selection', 'candidate', 'docs/a.md']);
		const ordinary = [
			{ owner: chosen, raw: selectedRequest },
			{ owner: omitted, raw: excludedRequest },
		];
		const selected = [
			{ owner: prerequisite, raw: '{"shared":true}' },
			ordinary[0] ?? { owner: '', raw: '{}' },
		];
		const options = {
			ordinary,
			selected,
			selectedOwners: [chosen],
			excludedOwners: [omitted],
			prerequisiteOwners: [prerequisite],
		};
		expect(() => requireControlledDispatch(options)).not.toThrow();
		expect(() => requireControlledDispatch({ ...options, prerequisiteOwners: [] })).toThrow(
			'inventory',
		);
		expect(() => requireControlledDispatch({ ...options, prerequisiteOwners: [omitted] })).toThrow(
			'overlap',
		);
	});
	test('retains every selected ID with its independently expected disposition', () => {
		const selectedIds = ['same-owner-a', 'same-owner-b', 'pending', 'stale'];
		const dispositions = [
			{ id: 'same-owner-a', kind: 'selected' },
			{ id: 'same-owner-b', kind: 'selected' },
			{ id: 'pending', kind: 'pending' },
			{ id: 'stale', kind: 'stale' },
		];
		const expectedKinds = new Map(dispositions.map(({ id, kind }) => [id, kind]));
		const options = { selectedIds, dispositions, expectedKinds };
		expect(() => requireCaseAccounting(options)).not.toThrow();
		expect(() =>
			requireCaseAccounting({ ...options, dispositions: dispositions.slice(0, 3) }),
		).toThrow('denominator');
		expect(() =>
			requireCaseAccounting({
				...options,
				dispositions: [...dispositions, dispositions[0] ?? { id: '', kind: '' }],
			}),
		).toThrow('Duplicate case');
		expect(() =>
			requireCaseAccounting({
				...options,
				dispositions: dispositions.map((value) =>
					value.id === 'pending' ? { id: value.id, kind: 'unmatched' } : value,
				),
			}),
		).toThrow('Wrong case disposition');
	});
	test('accepts one exact selected owner and an independently witnessed exclusion', () => {
		expect(() =>
			requireSelectedRequestParity({
				ordinary: report(
					requestRecord('same', 1, selectedRequest),
					requestRecord('same', 2, excludedRequest),
				),
				selected: report(requestRecord('same', 1, selectedRequest)),
				selectedOwners: [chosen],
				excludedOwners: [omitted],
				prerequisiteOwners: [],
			}),
		).not.toThrow();
	});
	test('rejects a reused span index for a different report', () => {
		const ordinary = report(
			requestRecord('same', 1, selectedRequest),
			requestRecord('same', 2, excludedRequest),
		);
		const selected = report(requestRecord('same', 1, selectedRequest));
		expect(() =>
			requireSelectedRequestParity({
				ordinary,
				selected,
				selectedOwners: [chosen],
				excludedOwners: [omitted],
				prerequisiteOwners: [],
				ordinaryReport: indexRequestReport(selected),
			}),
		).toThrow('Ordinary report index input differs');
	});
	test('joins a prepared digest to actual runtime request bytes', () => {
		const prepared = report(requestRecord('same', 1, selectedRequest));
		const runtime = report(
			requestRecord('same', 1, selectedRequest).replace(
				'"state":"prepared","status":"prepared"',
				'"state":"run"',
			),
		);
		expect(() => requirePreparedRuntimeParity(prepared, runtime, [chosen])).not.toThrow();
		expect(() =>
			requirePreparedRuntimeParity(prepared, runtime.replace('café', 'changed'), [chosen]),
		).toThrow('prepared/runtime input differs');
		expect(() => requirePreparedRuntimeParity(prepared, runtime, [omitted])).toThrow(
			'Prepared owner missing',
		);
		expect(() =>
			requirePreparedRuntimeParity(
				prepared,
				runtime.replace('"state":"run"', '"state":"not_sent"'),
				[chosen],
			),
		).toThrow('did not dispatch');
		expect(() =>
			requirePreparedRuntimeParity(
				prepared,
				runtime.replace(/"input_sha256":"[a-f0-9]+"/u, '"input_sha256":"incorrect"'),
				[chosen],
			),
		).toThrow('stored runtime input digest differs');
	});
	test('rejects post-dispatch filtering, inert baselines and changed numeric payload', () => {
		const ordinary = report(
			requestRecord('same', 1, selectedRequest),
			requestRecord('same', 2, excludedRequest),
		);
		const options = {
			ordinary,
			selected: report(requestRecord('same', 1, selectedRequest)),
			selectedOwners: [chosen],
			excludedOwners: [omitted],
			prerequisiteOwners: [],
		};
		expect(() => requireSelectedRequestParity({ ...options, selected: ordinary })).toThrow(
			'Excluded owner',
		);
		expect(() => requireSelectedRequestParity({ ...options, ordinary: options.selected })).toThrow(
			'ordinary dispatch witness',
		);
		expect(() =>
			requireSelectedRequestParity({
				...options,
				selected: report(
					requestRecord(
						'same',
						1,
						selectedRequest.replace('18446744073709551615', '18446744073709551614'),
					),
				),
			}),
		).toThrow('full request differs');
		expect(() => requireSelectedRequestParity({ ...options, selectedOwners: [] })).toThrow(
			'No selected owner witness',
		);
	});
});
