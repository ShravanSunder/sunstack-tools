/** Compare reports captured within one actual engine-owned revision snapshot. */
import assert from 'node:assert/strict';

import { compactJson, jsonSlice, requirePayloadBytes } from '../prepare/raw-json.ts';
import {
	arrayValue,
	objectValue,
	parseObject,
	textValue,
} from '../rulebook/rulebook-fixture-support.ts';
import { requireSelectedRequestParity, requestOwnerAt } from './case-dispatch-checks.ts';

export function requireSnapshotParity(
	ordinary: string,
	selected: string,
	bindingsRaw: string,
): object {
	const bindings = arrayValue(JSON.parse(bindingsRaw)).map(objectValue);
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
			const token = compactJson(jsonSlice(bindingsRaw, [index, 'lines', position]));
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
	const selectedRows = arrayValue(parseObject(selected)['requests']).map(objectValue);
	const selectedOwners = selectedRows.flatMap((row, index) =>
		objectValue(row['owner'])['kind'] === 'selection' ? [] : [requestOwnerAt(selected, index)],
	);
	const prerequisiteOwners = selectedRows.flatMap((row, index) =>
		objectValue(row['owner'])['kind'] === 'selection' ? [requestOwnerAt(selected, index)] : [],
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
					jsonSlice(selected, ['case_selection', 'excluded', index, 'lines', position]),
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
	});
	const ordinaryOwners = arrayValue(parseObject(ordinary)['requests']).map((_row, index) =>
		requestOwnerAt(ordinary, index),
	);
	const ordinaryRows = arrayValue(parseObject(ordinary)['requests']).map(objectValue);
	for (const owner of prerequisiteOwners) {
		const ordinaryIndex = ordinaryOwners.indexOf(owner);
		const selectedIndex = selectedRows.findIndex(
			(_row, index) => requestOwnerAt(selected, index) === owner,
		);
		assert.ok(
			ordinaryIndex >= 0 && selectedIndex >= 0,
			`Missing ordinary selection prerequisite: ${owner}`,
		);
		requirePayloadBytes({
			left: ordinary,
			leftPath: ['requests', ordinaryIndex, 'request'],
			right: selected,
			rightPath: ['requests', selectedIndex, 'request'],
			label: `${owner}: prerequisite request differs`,
		});
		assert.equal(
			ordinaryRows[ordinaryIndex]?.['input_sha256'],
			selectedRows[selectedIndex]?.['input_sha256'],
			`${owner}: prerequisite digest differs`,
		);
	}
	const ordinaryExecutionOwners = arrayValue(parseObject(ordinary)['requests'])
		.map(objectValue)
		.filter((row) => objectValue(row['owner'])['kind'] !== 'selection').length;
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
