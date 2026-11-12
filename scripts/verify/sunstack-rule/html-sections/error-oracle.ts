/** Strict report comparison with one version exception and source-declared recovered text. */
import assert from 'node:assert/strict';

import {
	arrayValue,
	numberValue,
	objectValue,
	textValue,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';

export function requireVersionOnlyBytes(before: Buffer, after: Buffer): void {
	const original = objectValue(JSON.parse(before.toString('utf8')));
	const current = objectValue(JSON.parse(after.toString('utf8')));
	assert.equal(original['version'], '0.14.0');
	assert.equal(current['version'], '0.14.1');
	// Replace only the root scalar's serialized occurrence, preserving every other byte.
	const prefix = '"version": "0.14.1"';
	assert.equal(after.toString('utf8').split(prefix).length, 2);
	assert.equal(
		after.toString('utf8').replace(prefix, '"version": "0.14.0"'),
		before.toString('utf8'),
	);
}

export function requireRecoveredControl(
	report: JsonObject,
	expected: Readonly<Record<string, { expected: string; cut: boolean; raw: string }>>,
): void {
	assert.equal(objectValue(report['usage'])['model_requests'], 0);
	const requests = arrayValue(report['requests']).map(objectValue);
	assert.equal(requests.length, Object.keys(expected).length);
	const seen = new Set<string>();
	for (const request of requests) {
		const piece = objectValue(
			objectValue(objectValue(report['piece_evidence'])[String(numberValue(request['id']))])[
				'piece'
			],
		);
		const file = textValue(objectValue(piece['where'])['file']);
		const control = expected[file];
		assert.ok(control, `Unexpected control ${file}`);
		assert.ok(!seen.has(file));
		seen.add(file);
		assert.equal(piece['text'], control.raw);
		assert.equal(objectValue(objectValue(piece['fields'])['full'])['value'], control.raw);
		const state = objectValue(objectValue(request['request'])['state']);
		assert.equal(state['full'], control.expected, file);
		assert.deepEqual(objectValue(piece['model_view'])['cut'], control.cut ? ['full'] : []);
		assert.deepEqual(state['cut-fields'] ?? [], control.cut ? ['full'] : []);
	}
}

export function reportRequests(
	report: JsonObject,
): Map<string, { piece: JsonObject; record: JsonObject; state: JsonObject }> {
	const records = new Map<string, { piece: JsonObject; record: JsonObject; state: JsonObject }>();
	for (const record of arrayValue(report['requests']).map(objectValue)) {
		const piece = objectValue(
			objectValue(objectValue(report['piece_evidence'])[String(numberValue(record['id']))])[
				'piece'
			],
		);
		const key = JSON.stringify(piece['where']);
		assert.ok(!records.has(key), 'Duplicate request owner');
		records.set(key, {
			piece,
			record,
			state: objectValue(objectValue(record['request'])['state']),
		});
	}
	return records;
}

/** Evidence listings alone cannot prove that recovered words reached an asking request. */
export function requireAmpersandAskingCoverage(report: JsonObject, listing: JsonObject): number {
	const errors = arrayValue(listing['pieces']).map(objectValue);
	assert.equal(errors.length, 10, 'Pinned real pages must retain every ERROR record');
	assert.equal(new Set(errors.map((error) => JSON.stringify(error['where']))).size, 5);
	assert.equal(new Set(errors.map((error) => objectValue(error['where'])['file'])).size, 3);
	const requests = arrayValue(report['requests']).map(objectValue);
	for (const error of errors) {
		const errorWhere = objectValue(error['where']);
		const errorSpan = objectValue(errorWhere['span']);
		const matches = requests.filter((record) => {
			const piece = objectValue(
				objectValue(objectValue(report['piece_evidence'])[String(numberValue(record['id']))])[
					'piece'
				],
			);
			const where = objectValue(piece['where']);
			const span = objectValue(where['span']);
			return (
				record['status'] === 'prepared' &&
				where['file'] === errorWhere['file'] &&
				numberValue(span['byte_start']) <= numberValue(errorSpan['byte_start']) &&
				numberValue(span['byte_end']) >= numberValue(errorSpan['byte_end']) &&
				JSON.stringify(span['source']) === JSON.stringify(errorSpan['source'])
			);
		});
		assert.ok(
			matches.length > 0,
			`No prepared asking owner contains ERROR ${JSON.stringify(errorWhere)}`,
		);
		assert.ok(
			matches.some((record) =>
				textValue(objectValue(objectValue(record['request'])['state'])['full']).includes(
					textValue(error['text']),
				),
			),
			'Containing prepared owner lost the actual ERROR words',
		);
	}
	return errors.length;
}
