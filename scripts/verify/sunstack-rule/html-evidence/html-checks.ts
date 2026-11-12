/** Independent owner/content comparisons against retained native CLI reports. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { isDeepStrictEqual } from 'node:util';

import {
	arrayValue,
	numberValue,
	objectValue,
	textValue,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { entityIds, requirementIds } from './html-corpus.ts';
import type { ExpectedControl } from './html-fixtures.ts';

export function savedPiece(report: JsonObject, row: JsonObject): JsonObject {
	return objectValue(
		objectValue(objectValue(report['piece_evidence'])[String(numberValue(row['id']))])['piece'],
	);
}
export function preparedPiece(report: JsonObject, request: JsonObject): JsonObject {
	const owner = objectValue(request['owner']);
	assert.equal(owner['kind'], 'rule');
	const pieces = Object.values(objectValue(report['piece_evidence']))
		.map((record) => objectValue(objectValue(record)['piece']))
		.filter((piece) => isDeepStrictEqual(piece['where'], owner['piece']));
	assert.equal(pieces.length, 1, 'Prepared owner does not identify one retained raw piece');
	const piece = pieces[0];
	assert.ok(piece);
	return piece;
}

export function requireNoDispatch(report: JsonObject): void {
	assert.equal(objectValue(report['usage'])['model_requests'], 0);
}

export function requireControls(report: JsonObject, expected: readonly ExpectedControl[]): void {
	requireNoDispatch(report);
	const rows = arrayValue(report['results']).map(objectValue);
	assert.equal(rows.length, expected.length, 'Control inventory changed');
	for (const control of expected) {
		const selected = rows.filter((row) => row['file'] === control.file);
		assert.equal(selected.length, 1, `Missing or repeated control ${control.file}`);
		assert.equal(selected[0]?.['outcome'], control.outcome, control.reason);
		if (control.file === 'pages/missing-file.html') {
			const row = selected[0];
			assert.ok(row);
			const targets = objectValue(objectValue(savedPiece(report, row)['fields'])['links']);
			assert.equal(targets['state'], 'targets');
			const values = arrayValue(targets['values']);
			assert.equal(values.length, 1);
			assert.equal(
				objectValue(values[0])['exists'],
				false,
				'Missing-file control accidentally points to a real file',
			);
		}
	}
}

export function requireRealCoverage(report: JsonObject, root: string): JsonObject[] {
	requireNoDispatch(report);
	const rows = arrayValue(report['results']).map(objectValue);
	assert.equal(rows.length, 15, 'Real R/E row inventory differs');
	const seen = new Set<string>();
	const account: JsonObject[] = [];
	for (const row of rows) {
		const piece = savedPiece(report, row);
		const fields = objectValue(piece['fields']);
		const identifier = textValue(objectValue(fields['identifier'])['value']);
		assert.ok(!seen.has(identifier), `Repeated real ID ${identifier}`);
		seen.add(identifier);
		const requirement = identifier.startsWith('R');
		assert.ok(
			(requirement ? [...requirementIds] : [...entityIds]).some((value) => value === identifier),
			`Unexpected ID ${identifier}`,
		);
		assert.equal(row['name'], requirement ? 'requirements-covered' : 'entities-bound');
		assert.equal(row['file'], 'pages/specification.html');
		assert.equal(row['outcome'], 'pass', `${identifier} was hand-confirmed in its binding cells`);
		const coverage = objectValue(fields['coverage']);
		assert.equal(coverage['state'], 'texts');
		assert.equal(
			arrayValue(coverage['values']).length,
			4,
			'Document did not collect all four binding rows',
		);
		for (const spanValue of arrayValue(coverage['spans'])) {
			const span = objectValue(spanValue);
			assert.equal(
				objectValue(span['source'])['path'],
				requirement ? 'pages/verification-design.html' : 'pages/program-design.html',
			);
		}
		requireRawSpan(piece, root);
		account.push({
			identifier,
			result_id: numberValue(row['id']),
			outcome: textValue(row['outcome']),
			file: textValue(row['file']),
			coverage: arrayValue(coverage['values']),
		});
	}
	assert.deepEqual([...seen].toSorted(), [...requirementIds, ...entityIds].toSorted());
	return account;
}

export function requireRawSpan(piece: JsonObject, root: string): void {
	const location = objectValue(piece['where']);
	const span = objectValue(location['span']);
	const source = filesystem.readFileSync(path.join(root, textValue(location['file'])));
	const raw = source
		.subarray(numberValue(span['byte_start']), numberValue(span['byte_end']))
		.toString('utf8');
	assert.equal(piece['text'], raw, 'Location does not delimit original raw source');
	assert.ok(raw.startsWith('<'), 'Raw HTML text lost markup');
}

export function requireAsking(report: JsonObject, root: string, expected: string): void {
	requireNoDispatch(report);
	const requests = arrayValue(report['requests']).map(objectValue);
	assert.equal(requests.length, 1, 'Prepared asking inventory differs');
	const request = requests[0];
	assert.ok(request);
	assert.equal(request['status'], 'prepared');
	assert.equal(
		objectValue(objectValue(request['request'])['state'])['body'],
		expected,
		'Prepared HTML text differs from authored expectation',
	);
	assert.deepEqual(
		arrayValue(report['results']),
		[],
		'Unanswered asking request gained a settled result',
	);
	const piece = preparedPiece(report, request);
	requireRawSpan(piece, root);
	assert.equal(
		objectValue(objectValue(piece['fields'])['body'])['value'],
		piece['text'],
		'Mechanical body lost original markup',
	);
}

export const realAskingText =
	'R1 / U1C1 Preserve capability grouping/picker, equal-size four-view controls, selected list/detail navigation, previous/next, related docs and nested call views. Same supplied facts remain readable in each.V1 Same selected 42-record lab source, desktop and390CSSpx, both themes; missing facts or changed selected form fails. Permitted natural wrapping passes.';
