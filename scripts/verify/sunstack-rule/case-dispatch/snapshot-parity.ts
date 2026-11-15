/** Compare reports captured within one actual engine-owned revision snapshot. */
import assert from 'node:assert/strict';

import { compactJson, indexJson } from '../prepare/raw-json.ts';
import {
	arrayValue,
	objectValue,
	parseObject,
	textValue,
} from '../rulebook/rulebook-fixture-support.ts';
import { indexRequestReport, requireSelectedRequestParity } from './case-dispatch-checks.ts';

export function requireSnapshotParity(
	ordinary: string,
	selected: string,
	bindingsRaw: string,
): object {
	const bindings = arrayValue(JSON.parse(bindingsRaw)).map(objectValue);
	const bindingSpans = indexJson(bindingsRaw);
	const ids = new Set<string>();
	const allowed = new Set<string>();
	for (const [index, binding] of bindings.entries()) {
		const id = textValue(binding['id']);
		assert.ok(id.length > 0 && !ids.has(id), `Missing or repeated selected binding ID: ${id}`);
		ids.add(id);
		if (binding['kind'] === 'filter') {
			allowed.add(
				JSON.stringify(['filter', textValue(binding['filter']), textValue(binding['folder'])]),
			);
			continue;
		}
		assert.equal(binding['kind'], 'rule', 'Unknown selected binding kind');
		const line = (position: number): string => {
			const token = compactJson(bindingSpans.slice([index, 'lines', position]));
			assert.match(token, /^(0|[1-9][0-9]*)$/u);
			return token;
		};
		allowed.add(
			JSON.stringify([
				'rule',
				textValue(binding['rule']),
				textValue(binding['file']),
				line(0),
				line(1),
			]),
		);
	}
	const selectedIndex = indexRequestReport(selected);
	const ordinaryReport = indexRequestReport(ordinary);
	const selectedRows = selectedIndex.rows;
	const selectedOwners = selectedRows.flatMap((row, index) =>
		objectValue(row['owner'])['kind'] === 'selection' ? [] : [selectedIndex.ownerAt(index)],
	);
	const prerequisiteOwners = selectedRows.flatMap((row, index) =>
		objectValue(row['owner'])['kind'] === 'selection' ? [selectedIndex.ownerAt(index)] : [],
	);
	const selection = objectValue(parseObject(selected)['case_selection']);
	const dispositions = arrayValue(selection['bindings'])
		.map(objectValue)
		.map((entry) => ({
			id: textValue(objectValue(entry['binding'])['id']),
			kind: textValue(objectValue(entry['outcome'])['kind']),
		}));
	assert.deepEqual(
		dispositions.map((entry) => entry.id).toSorted(),
		[...ids].toSorted(),
		'Selected binding denominator differs',
	);
	const excluded = arrayValue(selection['excluded'])
		.map(objectValue)
		.map((owner, index) => {
			if (owner['kind'] === 'filter')
				return JSON.stringify(['filter', textValue(owner['filter']), textValue(owner['folder'])]);
			assert.equal(owner['kind'], 'rule', 'Unknown excluded execution owner');
			const line = (position: number): string => {
				const token = compactJson(
					selectedIndex.slice(['case_selection', 'excluded', index, 'lines', position]),
				);
				assert.match(token, /^(0|[1-9][0-9]*)$/u);
				return token;
			};
			return JSON.stringify([
				'rule',
				textValue(owner['rule']),
				textValue(owner['file']),
				line(0),
				line(1),
			]);
		});
	for (const owner of excluded)
		assert.ok(
			!selectedOwners.includes(owner),
			`Excluded execution owner appeared in selected report: ${owner}`,
		);
	assert.ok(selectedOwners.length > 0, 'Snapshot produced no selected execution request');
	for (const owner of selectedOwners)
		assert.ok(
			allowed.has(owner),
			`Execution owner is not selected by any supplied binding: ${owner}`,
		);
	requireSelectedRequestParity({
		ordinary,
		selected,
		selectedOwners,
		excludedOwners: [],
		prerequisiteOwners,
		ordinaryReport,
		selectedReport: selectedIndex,
	});
	const ordinaryRows = ordinaryReport.rows;
	const ordinaryOwners = ordinaryRows.map((_row, index) => ordinaryReport.ownerAt(index));
	for (const owner of prerequisiteOwners) {
		const ordinaryIndex = ordinaryOwners.indexOf(owner);
		const selectedRowIndex = selectedRows.findIndex(
			(_row, index) => selectedIndex.ownerAt(index) === owner,
		);
		assert.ok(
			ordinaryIndex >= 0 && selectedRowIndex >= 0,
			`Missing ordinary selection prerequisite: ${owner}`,
		);
		assert.equal(
			compactJson(ordinaryReport.slice(['requests', ordinaryIndex, 'request'])),
			compactJson(selectedIndex.slice(['requests', selectedRowIndex, 'request'])),
			`${owner}: prerequisite request differs`,
		);
		assert.equal(
			ordinaryRows[ordinaryIndex]?.['input_sha256'],
			selectedRows[selectedRowIndex]?.['input_sha256'],
			`${owner}: prerequisite digest differs`,
		);
	}
	const ordinaryExecutionOwners = ordinaryRows.filter(
		(row) => objectValue(row['owner'])['kind'] !== 'selection',
	).length;
	return {
		selectedOwners,
		prerequisiteOwners,
		ordinaryOwners,
		excludedOwners: excluded,
		selectedExecutionOwners: selectedOwners.length,
		ordinaryExecutionOwners,
		owners: selectedOwners,
		bindingIds: [...ids],
		dispositions,
		parity: 'matched_raw_request_and_native_input_sha256',
	};
}
