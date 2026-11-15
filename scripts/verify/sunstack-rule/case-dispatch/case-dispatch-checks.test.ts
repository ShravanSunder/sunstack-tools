import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { describe, expect, test } from 'vitest';

import {
	attributeControlledCaptures,
	requireCaseAccounting,
	requireControlledDispatch,
	requireManifestSelection,
	requireSelectedRequestParity,
	requireUniqueOwnerBindings,
	requestOwnerAt,
} from './case-dispatch-checks.ts';
import { readCorpusSelections, scoreSelectionArguments } from './case-dispatch-corpus.ts';
import { createCaseDispatchFixture } from './case-dispatch-fixture.ts';

function requestRecord(rule: string, line: number, rawRequest: string): string {
	const digest = createHash('sha256').update(rawRequest).digest('hex');
	return `{"id":${line},"owner":{"kind":"rule","rule":"${rule}","piece":{"file":"docs/a.md","first_line":${line},"last_line":${line}}},"state":"prepared","status":"prepared","input_sha256":"${digest}","request":${rawRequest}}`;
}
function report(...records: string[]): string {
	return `{"mode":"prepare","requests":[${records.join(',')}]}`;
}
function http(request: string): string {
	return `{"model":"test/model",${request.slice(1)}`;
}
const chosen = JSON.stringify(['rule', 'same', 'docs/a.md', '1', '1']);
const omitted = JSON.stringify(['rule', 'same', 'docs/a.md', '2', '2']);
const selectedRequest =
	'{"state":{"text":"café","wide":18446744073709551615},"questions":{"a":{"type":"noul"}}}';
const excludedRequest = '{"state":{"text":"exclude"},"questions":{"a":{"type":"noul"}}}';

describe('selected request parity observation', () => {
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
