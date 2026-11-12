import assert from 'node:assert/strict';

import { test } from 'vitest';

import { objectValue, type JsonObject } from '../rulebook/rulebook-fixture-support.ts';
import { requireFilterProjection, requireSelectionProjection } from './asking-paths.ts';
import { askingPathHtml, askingPathText } from './fixtures.ts';

function selectionReport(): JsonObject {
	return {
		usage: { model_requests: 0 },
		requests: [
			{
				owner: { kind: 'selection' },
				status: 'prepared',
				request: {
					state: {
						candidates: { ['c1']: { full: askingPathText } },
						shared: { context: [askingPathText] },
					},
				},
				selection_evidence: {
					candidates: [{ fields: { full: { state: 'text', value: askingPathHtml } } }],
					shared: { fields: { context: { state: 'texts', values: [askingPathHtml] } } },
				},
			},
		],
	};
}
function filterReport(): JsonObject {
	return {
		usage: { model_requests: 0 },
		requests: [
			{
				id: 1,
				owner: { kind: 'filter' },
				status: 'prepared',
				request: { state: { full: askingPathText, 'unavailable-fields': {} } },
			},
		],
		piece_evidence: {
			'1': {
				piece: { text: askingPathHtml, fields: { full: { state: 'text', value: askingPathHtml } } },
			},
		},
	};
}
function state(report: JsonObject): JsonObject {
	const records = report['requests'];
	assert.ok(Array.isArray(records));
	return objectValue(objectValue(objectValue(records[0])['request'])['state']);
}
test('requires projected selection candidates and a separately projected shared view', () => {
	requireSelectionProjection(selectionReport());
	const rawCandidate = selectionReport();
	state(rawCandidate)['candidates'] = { ['c1']: { full: askingPathHtml } };
	assert.throws(() => requireSelectionProjection(rawCandidate));
	const rawShared = selectionReport();
	state(rawShared)['shared'] = { context: [askingPathHtml] };
	assert.throws(() => requireSelectionProjection(rawShared));
});
test('rejects selected raw evidence replaced by its asking projection', () => {
	const report = selectionReport();
	const records = report['requests'];
	assert.ok(Array.isArray(records));
	objectValue(objectValue(records[0])['selection_evidence'])['candidates'] = [
		{ fields: { full: { state: 'text', value: askingPathText } } },
	];
	assert.throws(() => requireSelectionProjection(report));
});
test('requires projected filter state while retaining raw declared reads', () => {
	requireFilterProjection(filterReport());
	const rawRequest = filterReport();
	state(rawRequest)['full'] = askingPathHtml;
	assert.throws(() => requireFilterProjection(rawRequest));
	const rawChanged = filterReport();
	objectValue(objectValue(objectValue(rawChanged['piece_evidence'])['1'])['piece'])['fields'] = {
		full: { state: 'text', value: askingPathText },
	};
	assert.throws(() => requireFilterProjection(rawChanged));
});
test('rejects a supposedly prepared asking path that dispatched to a model', () => {
	const report = selectionReport();
	objectValue(report['usage'])['model_requests'] = 1;
	assert.throws(() => requireSelectionProjection(report));
});
