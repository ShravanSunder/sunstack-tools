/** Report/capture comparison only; expected request contents never come from a builder here. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import {
	arrayValue,
	objectValue,
	parseObject,
	textValue,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { compactJson, jsonSlice, requirePayloadBytes } from './raw-json.ts';

export function requestOwner(record: JsonObject): string {
	const owner = objectValue(record['owner']);
	if (owner['kind'] === 'rule') {
		const piece = objectValue(owner['piece']);
		return JSON.stringify([
			'rule',
			owner['rule'],
			piece['file'],
			piece['first_line'],
			piece['last_line'],
		]);
	}
	if (owner['kind'] === 'filter')
		return JSON.stringify(['filter', owner['filter'], owner['folder']]);
	assert.equal(owner['kind'], 'selection', 'Unknown saved request owner');
	return JSON.stringify(['selection', owner['selection_kind'], owner['file']]);
}
function requireReplayedSelection(
	raw: string,
	index: number,
	record: JsonObject,
	source: string,
): void {
	const owner = objectValue(record['owner']);
	assert.equal(owner['kind'], 'selection', 'Only selection answers may replay');
	assert.equal(record['status'], 'from_report');
	const recorded = arrayValue(parseObject(source)['requests']).map(objectValue);
	const sourceIndex = recorded.findIndex((item) => item['id'] === record['source_request_id']);
	assert.ok(sourceIndex >= 0, 'Replay provenance points to no source request');
	const original = recorded[sourceIndex];
	assert.ok(original);
	assert.equal(requestOwner(record), requestOwner(original));
	assert.equal(objectValue(original['outcome'])['kind'], 'completed');
	assert.equal(record['resolved_model'], original['resolved_model']);
	assert.equal(record['input_sha256'], original['input_sha256']);
	for (const field of ['request', 'outcome', 'selection_evidence'])
		requirePayloadBytes({
			left: raw,
			leftPath: ['requests', index, field],
			right: source,
			rightPath: ['requests', sourceIndex, field],
			label: `Replayed ${field} differs from recorded selection`,
		});
}
export function preparedInventory(raw: string, replayedFrom?: string): Map<string, number> {
	const report = parseObject(raw);
	assert.equal(report['mode'], 'prepare');
	const result = new Map<string, number>();
	arrayValue(report['requests'])
		.map(objectValue)
		.forEach((record, index) => {
			if (record['state'] === 'from_report') {
				assert.ok(replayedFrom, 'Unexpected replayed selection');
				requireReplayedSelection(raw, index, record, replayedFrom);
			} else {
				assert.equal(record['state'], 'prepared');
				assert.equal(record['status'], 'prepared');
				assert.ok(!('outcome' in record), 'Prepared record invented an outcome');
			}
			assert.equal(
				record['input_sha256'],
				createHash('sha256')
					.update(compactJson(jsonSlice(raw, ['requests', index, 'request'])))
					.digest('hex'),
				'Prepared input hash differs from persisted request bytes',
			);
			assert.ok(!('answers' in record), 'Prepared record invented answers');
			assert.ok(
				!('answers' in objectValue(record['request'])),
				'Prepared payload invented answers',
			);
			const owner = requestOwner(record);
			assert.ok(!result.has(owner), `Duplicate prepared owner ${owner}`);
			result.set(owner, index);
		});
	return result;
}
export function requireRequestParity(options: {
	prepared: string;
	ordinary: string;
	captures: readonly string[];
	expectedPrepared: number;
	replayedFrom?: string;
}): void {
	const prepared = preparedInventory(options.prepared, options.replayedFrom);
	assert.equal(prepared.size, options.expectedPrepared, 'Prepared request inventory differs');
	const ordinary = parseObject(options.ordinary);
	assert.ok(!('mode' in ordinary), 'Ordinary report gained prepare-only mode');
	assert.ok(!('needs_answer' in ordinary), 'Ordinary report gained prepare-only dependencies');
	const records = arrayValue(ordinary['requests']).map(objectValue);
	assert.equal(records.length, options.captures.length, 'Ordinary report/capture count differs');
	const seen = new Set<string>();
	const consumed = new Set<number>();
	for (const [index, record] of records.entries()) {
		const owner = requestOwner(record);
		assert.ok(!seen.has(owner), `Duplicate ordinary owner ${owner}`);
		seen.add(owner);
		const preparedIndex = prepared.get(owner);
		assert.notEqual(preparedIndex, undefined, `Missing prepared owner ${owner}`);
		assert.equal(typeof preparedIndex, 'number');
		if (preparedIndex === undefined) throw new Error('Missing prepared owner');
		for (const field of ['state', 'questions'])
			requirePayloadBytes({
				left: options.prepared,
				leftPath: ['requests', preparedIndex, 'request', field],
				right: options.ordinary,
				rightPath: ['requests', index, 'request', field],
				label: `${owner}: prepared ${field} differs from saved request`,
			});
		const capture = options.captures.findIndex(
			(raw, position) =>
				!consumed.has(position) &&
				['state', 'questions'].every(
					(field) =>
						compactJson(jsonSlice(raw, [field])) ===
						compactJson(jsonSlice(options.ordinary, ['requests', index, 'request', field])),
				),
		);
		assert.ok(capture >= 0, `${owner}: no actual HTTP payload matches the saved request`);
		consumed.add(capture);
	}
	assert.equal(consumed.size, options.captures.length, 'An HTTP request was never accounted');
}
export function requirePreparedGate(raw: string, rule: string): void {
	const requests = arrayValue(parseObject(raw)['requests']).map(objectValue);
	const gated = requests.filter((record) => objectValue(record['owner'])['rule'] === rule);
	assert.ok(gated.length > 0, 'No gated rule request was prepared');
	for (const record of gated)
		assert.deepEqual(record['gate'], { filter: 'category', labels: ['eligible'] });
}
export function requireSelectionWaits(raw: string): void {
	const report = parseObject(raw);
	preparedInventory(raw);
	const requests = arrayValue(report['requests']).map(objectValue);
	const waits = arrayValue(report['needs_answer']).map(objectValue);
	assert.ok(waits.length > 0, 'Selected kinds silently lost dependent work');
	const kinds = new Set(waits.map((wait) => textValue(wait['kind'])));
	assert.ok(
		kinds.has('workflow_section') && kinds.has('step'),
		'Selected or within kind missing from dependency list',
	);
	for (const wait of waits) {
		const dependency = objectValue(wait['waits_on']);
		assert.equal(dependency['selection_kind'], 'workflow_section');
		assert.equal(wait['file'], 'skills/practices-research/SKILL.md');
		assert.ok(
			requests.some((record) => {
				const owner = objectValue(record['owner']);
				return (
					record['id'] === dependency['request_id'] &&
					owner['kind'] === 'selection' &&
					owner['selection_kind'] === dependency['selection_kind'] &&
					owner['file'] === wait['file']
				);
			}),
			'Dangling selection dependency',
		);
	}
}
