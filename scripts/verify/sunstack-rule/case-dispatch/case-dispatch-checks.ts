/** Compare saved request owners and raw payloads; no product request is constructed here. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import { compactJson, indexJson } from '../prepare/raw-json.ts';
import {
	arrayValue,
	objectValue,
	parseObject,
	textValue,
} from '../rulebook/rulebook-fixture-support.ts';

/** Reject score IDs that are missing or overlap the caller's held/pending categories. */
export function requireManifestSelection(options: {
	readonly scoreIds: readonly string[];
	readonly heldIds: readonly string[];
	readonly pendingIds: readonly string[];
	readonly unmatchedIds: readonly string[];
	readonly notCollectibleIds: readonly string[];
	readonly caseIds: readonly string[];
}): void {
	const all = new Set<string>();
	for (const ids of [
		options.scoreIds,
		options.heldIds,
		options.pendingIds,
		options.unmatchedIds,
		options.notCollectibleIds,
	]) {
		for (const id of ids) {
			assert.ok(id.length > 0 && !all.has(id), `Duplicate or overlapping manifest ID: ${id}`);
			all.add(id);
		}
	}
	const caseIds = new Set(options.caseIds);
	assert.equal(caseIds.size, options.caseIds.length, 'Duplicate case-file ID');
	for (const id of options.scoreIds)
		assert.ok(caseIds.has(id), `Scored ID absent from case file: ${id}`);
}

/** Reuse one parsed report and original byte-span index for all owner comparisons. */
export function indexRequestReport(raw: string): {
	readonly raw: string;
	readonly rows: ReturnType<typeof objectValue>[];
	readonly mode: unknown;
	readonly slice: (path: readonly (string | number)[]) => string;
	readonly ownerAt: (index: number) => string;
} {
	const report = parseObject(raw);
	const rows = arrayValue(report['requests']).map(objectValue);
	const spans = indexJson(raw);
	return {
		raw,
		rows,
		mode: report['mode'],
		slice: spans.slice,
		ownerAt: (index) => ownerAt(rows, spans.slice, index),
	};
}
function ownerAt(
	rows: ReturnType<typeof objectValue>[],
	slice: (path: readonly (string | number)[]) => string,
	index: number,
): string {
	const row = rows[index];
	assert.ok(row, 'Missing indexed request');
	const owner = objectValue(row['owner']);
	if (owner['kind'] === 'rule') {
		const piece = objectValue(owner['piece']);
		const integer = (name: string): string => {
			const token = compactJson(slice(['requests', index, 'owner', 'piece', name]));
			assert.match(token, /^(0|[1-9][0-9]*)$/u, 'Owner line is not an unsigned integer token');
			return token;
		};
		return JSON.stringify([
			'rule',
			textValue(owner['rule']),
			textValue(piece['file']),
			integer('first_line'),
			integer('last_line'),
		]);
	}
	if (owner['kind'] === 'filter')
		return JSON.stringify(['filter', textValue(owner['filter']), textValue(owner['folder'])]);
	assert.equal(owner['kind'], 'selection', 'Unknown request owner');
	return JSON.stringify([
		'selection',
		textValue(owner['selection_kind']),
		textValue(owner['file']),
	]);
}
/** Keep line identifiers as original integer tokens, not rounded JavaScript numbers. */
export function requestOwnerAt(raw: string, index: number): string {
	return indexRequestReport(raw).ownerAt(index);
}

function preparedInventory(raw: string, indexed = indexRequestReport(raw)): Map<string, number> {
	assert.equal(indexed.mode, 'prepare');
	const inventory = new Map<string, number>();
	for (const [index, row] of indexed.rows.entries()) {
		assert.equal(row['state'], 'prepared');
		assert.equal(row['status'], 'prepared');
		assert.ok(
			!('answers' in row) && !('answers' in objectValue(row['request'])),
			'Prepared request invented answers',
		);
		const payload = compactJson(indexed.slice(['requests', index, 'request']));
		assert.equal(
			row['input_sha256'],
			createHash('sha256').update(payload).digest('hex'),
			'Prepared input hash differs',
		);
		const owner = indexed.ownerAt(index);
		assert.ok(!inventory.has(owner), `Duplicate prepared owner ${owner}`);
		inventory.set(owner, index);
	}
	return inventory;
}

/** Connect the persisted prepare digest to the actual saved runtime request for each owner. */
export function requirePreparedRuntimeParity(
	prepared: string,
	runtime: string,
	owners: readonly string[],
): void {
	assert.ok(
		owners.length > 0 && new Set(owners).size === owners.length,
		'Prepared/runtime owners must be nonempty and unique',
	);
	const preparedReport = indexRequestReport(prepared);
	const runtimeReport = indexRequestReport(runtime);
	const preparedOwners = preparedInventory(prepared, preparedReport);
	const runtimeRows = runtimeReport.rows;
	for (const owner of owners) {
		const preparedIndex = preparedOwners.get(owner);
		assert.notEqual(preparedIndex, undefined, `Prepared owner missing: ${owner}`);
		if (preparedIndex === undefined) throw new Error('Missing prepared owner');
		const runtimeIndexes = runtimeRows.flatMap((_row, index) =>
			runtimeReport.ownerAt(index) === owner ? [index] : [],
		);
		assert.equal(runtimeIndexes.length, 1, `Runtime owner missing or duplicated: ${owner}`);
		const runtimeIndex = runtimeIndexes[0];
		if (runtimeIndex === undefined) throw new Error('Missing runtime owner');
		assert.equal(
			runtimeRows[runtimeIndex]?.['state'],
			'run',
			`Runtime owner did not dispatch: ${owner}`,
		);
		assert.equal(
			compactJson(preparedReport.slice(['requests', preparedIndex, 'request'])),
			compactJson(runtimeReport.slice(['requests', runtimeIndex, 'request'])),
			`${owner}: prepared/runtime input differs`,
		);
		const digest = createHash('sha256')
			.update(compactJson(runtimeReport.slice(['requests', runtimeIndex, 'request'])))
			.digest('hex');
		assert.equal(
			digest,
			preparedReport.rows[preparedIndex]?.['input_sha256'],
			`${owner}: runtime input digest differs`,
		);
		const runtimeDigest = runtimeRows[runtimeIndex]?.['input_sha256'];
		if (runtimeDigest !== undefined)
			assert.equal(runtimeDigest, digest, `${owner}: stored runtime input digest differs`);
	}
}

/** Bind raw HTTP captures to their one saved request using model and complete request fields. */
export function attributeControlledCaptures(
	reportRaw: string,
	captures: readonly string[],
): { readonly owner: string; readonly raw: string }[] {
	const report = indexRequestReport(reportRaw);
	const rows = report.rows;
	assert.equal(rows.length, captures.length, 'Saved request and HTTP capture counts differ');
	const consumed = new Set<number>();
	return captures.map((raw) => {
		const capture = parseObject(raw);
		const captureSpans = indexJson(raw);
		const matches = rows.flatMap((row, index) => {
			if (consumed.has(index) || row['requested_model'] !== capture['model']) return [];
			for (const field of ['state', 'questions']) {
				if (
					compactJson(report.slice(['requests', index, 'request', field])) !==
					compactJson(captureSpans.slice([field]))
				)
					return [];
			}
			return [index];
		});
		assert.equal(matches.length, 1, 'HTTP capture has missing or ambiguous saved owner');
		const index = matches[0];
		assert.notEqual(index, undefined, 'Missing HTTP owner index');
		if (index === undefined) throw new Error('Missing HTTP owner index');
		consumed.add(index);
		return { owner: report.ownerAt(index), raw };
	});
}

/** Check the caller-supplied denominator independently of the product's aggregate counters. */
export function requireCaseAccounting(options: {
	readonly selectedIds: readonly string[];
	readonly dispositions: readonly { readonly id: string; readonly kind: string }[];
	readonly expectedKinds: ReadonlyMap<string, string>;
}): void {
	const selected = new Set(options.selectedIds);
	assert.equal(selected.size, options.selectedIds.length, 'Duplicate selected case ID');
	assert.equal(options.expectedKinds.size, selected.size, 'Expected denominator differs');
	const actual = new Map<string, string>();
	for (const disposition of options.dispositions) {
		assert.ok(!actual.has(disposition.id), `Duplicate case disposition: ${disposition.id}`);
		actual.set(disposition.id, disposition.kind);
	}
	assert.deepEqual(
		[...actual.keys()].toSorted(),
		[...selected].toSorted(),
		'Selected case denominator differs',
	);
	for (const id of selected) {
		assert.ok(options.expectedKinds.has(id), `Expected disposition missing: ${id}`);
		assert.equal(actual.get(id), options.expectedKinds.get(id), `Wrong case disposition: ${id}`);
	}
}

/** Empty bar provenance produces a meaningful unassessed cases exit, even when cases are right. */
export function requireUnassessedBars(report: ReturnType<typeof parseObject>): void {
	const bars = arrayValue(report['bars']).map(objectValue);
	assert.ok(bars.length > 0, 'Expected authored thresholds with unassessed bars');
	for (const bar of bars) {
		assert.equal(bar['state'], 'unassessed');
		assert.equal(bar['reason'], 'bar has no recorded development cases');
		assert.equal(bar['value'], null);
		assert.deepEqual(bar['set_from'], []);
	}
}

export function requireCleanCaseRun(report: ReturnType<typeof parseObject>): void {
	const runs = arrayValue(report['runs']).map(objectValue);
	assert.equal(runs.length, 1, 'Expected one historical run');
	assert.equal(objectValue(objectValue(runs[0]?.['report'])['status'])['kind'], 'clean');
}

/** Keep every matched case ID even when multiple IDs share a single execution owner. */
export function requireUniqueOwnerBindings(options: {
	readonly expected: ReadonlyMap<string, string>;
	readonly actual: readonly { readonly id: string; readonly owner: string }[];
	readonly dispatchedOwners: readonly string[];
}): void {
	const bindings = new Map<string, string>();
	for (const binding of options.actual) {
		assert.ok(!bindings.has(binding.id), `Duplicate matched case ID: ${binding.id}`);
		bindings.set(binding.id, binding.owner);
	}
	assert.deepEqual(
		[...bindings.keys()].toSorted(),
		[...options.expected.keys()].toSorted(),
		'Matched case IDs differ',
	);
	for (const [id, owner] of options.expected)
		assert.equal(bindings.get(id), owner, `Matched owner differs: ${id}`);
	const dispatched = new Set(options.dispatchedOwners);
	assert.equal(
		dispatched.size,
		options.dispatchedOwners.length,
		'Execution owner dispatched more than once',
	);
	assert.deepEqual(
		[...dispatched].toSorted(),
		[...new Set(options.expected.values())].toSorted(),
		'Execution owner inventory differs',
	);
}

/** Compare independently attributed raw HTTP captures, including zero excluded calls. */
export function requireControlledDispatch(options: {
	readonly ordinary: readonly { readonly owner: string; readonly raw: string }[];
	readonly selected: readonly { readonly owner: string; readonly raw: string }[];
	readonly selectedOwners: readonly string[];
	readonly excludedOwners: readonly string[];
	readonly prerequisiteOwners: readonly string[];
}): void {
	const expected = new Set(options.selectedOwners);
	const excluded = new Set(options.excludedOwners);
	const prerequisites = new Set(options.prerequisiteOwners);
	assert.ok(expected.size > 0, 'No selected HTTP witness');
	assert.equal(expected.size, options.selectedOwners.length, 'Duplicate expected selected owner');
	assert.equal(excluded.size, options.excludedOwners.length, 'Duplicate expected excluded owner');
	assert.equal(
		prerequisites.size,
		options.prerequisiteOwners.length,
		'Duplicate prerequisite owner',
	);
	for (const owner of expected)
		assert.ok(!excluded.has(owner) && !prerequisites.has(owner), 'Owner categories overlap');
	for (const owner of excluded) assert.ok(!prerequisites.has(owner), 'Owner categories overlap');
	const index = (captures: typeof options.ordinary): Map<string, string> => {
		const result = new Map<string, string>();
		for (const capture of captures) {
			assert.ok(!result.has(capture.owner), `Duplicate HTTP owner ${capture.owner}`);
			result.set(capture.owner, capture.raw);
		}
		return result;
	};
	const ordinary = index(options.ordinary);
	const selected = index(options.selected);
	for (const owner of excluded) {
		assert.ok(ordinary.has(owner), `Excluded owner has no ordinary HTTP witness: ${owner}`);
		assert.ok(!selected.has(owner), `Excluded owner dispatched HTTP: ${owner}`);
	}
	assert.deepEqual(
		[...selected.keys()].toSorted(),
		[...expected, ...prerequisites].toSorted(),
		'Selected HTTP owner inventory differs',
	);
	for (const owner of expected) {
		assert.ok(ordinary.has(owner), `Selected owner has no ordinary HTTP reference: ${owner}`);
		assert.equal(selected.get(owner), ordinary.get(owner), `${owner}: full HTTP body differs`);
	}
}

/** Require a nonempty, exact selected owner set and byte-preserving ordinary parity. */
export function requireSelectedRequestParity(options: {
	readonly ordinary: string;
	readonly selected: string;
	readonly selectedOwners: readonly string[];
	readonly excludedOwners: readonly string[];
	readonly prerequisiteOwners: readonly string[];
	readonly ordinaryReport?: ReturnType<typeof indexRequestReport>;
	readonly selectedReport?: ReturnType<typeof indexRequestReport>;
}): void {
	const expected = new Set(options.selectedOwners);
	const excluded = new Set(options.excludedOwners);
	const prerequisites = new Set(options.prerequisiteOwners);
	assert.ok(expected.size > 0, 'No selected owner witness');
	assert.equal(expected.size, options.selectedOwners.length, 'Duplicate expected selected owner');
	assert.equal(excluded.size, options.excludedOwners.length, 'Duplicate expected excluded owner');
	assert.equal(
		prerequisites.size,
		options.prerequisiteOwners.length,
		'Duplicate prerequisite owner',
	);
	for (const owner of expected)
		assert.ok(!excluded.has(owner) && !prerequisites.has(owner), 'Owner categories overlap');
	for (const owner of excluded) assert.ok(!prerequisites.has(owner), 'Owner categories overlap');
	const ordinaryReport = options.ordinaryReport ?? indexRequestReport(options.ordinary);
	const selectedReport = options.selectedReport ?? indexRequestReport(options.selected);
	assert.equal(ordinaryReport.raw, options.ordinary, 'Ordinary report index input differs');
	assert.equal(selectedReport.raw, options.selected, 'Selected report index input differs');
	const ordinary = preparedInventory(options.ordinary, ordinaryReport);
	const selected = preparedInventory(options.selected, selectedReport);
	for (const owner of excluded) {
		assert.ok(ordinary.has(owner), `Excluded owner has no ordinary dispatch witness: ${owner}`);
		assert.ok(!selected.has(owner), `Excluded owner was prepared: ${owner}`);
	}
	const allowed = new Set([...expected, ...prerequisites]);
	assert.deepEqual(
		[...selected.keys()].toSorted(),
		[...allowed].toSorted(),
		'Selected owner inventory differs',
	);
	const ordinaryRows = ordinaryReport.rows;
	const selectedRows = selectedReport.rows;
	for (const owner of expected) {
		const oldIndex = ordinary.get(owner);
		const newIndex = selected.get(owner);
		assert.notEqual(oldIndex, undefined, `Selected owner missing in ordinary reference: ${owner}`);
		assert.notEqual(newIndex, undefined, `Selected owner missing: ${owner}`);
		if (oldIndex === undefined || newIndex === undefined) throw new Error('Missing owner index');
		assert.equal(
			compactJson(ordinaryReport.slice(['requests', oldIndex, 'request'])),
			compactJson(selectedReport.slice(['requests', newIndex, 'request'])),
			`${owner}: full request differs`,
		);
		const oldRow = ordinaryRows[oldIndex];
		const newRow = selectedRows[newIndex];
		assert.ok(oldRow && newRow, 'Missing indexed request');
		assert.equal(
			textValue(newRow['input_sha256']),
			textValue(oldRow['input_sha256']),
			`${owner}: input digest differs`,
		);
		const selectedBytes = compactJson(selectedReport.slice(['requests', newIndex, 'request']));
		assert.equal(
			newRow['input_sha256'],
			createHash('sha256').update(selectedBytes).digest('hex'),
			`${owner}: incorrect input digest`,
		);
	}
}
