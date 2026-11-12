import assert from 'node:assert/strict';

import { test } from 'vitest';

import { requireControls } from './html-checks.ts';
import type { ExpectedControl } from './html-fixtures.ts';

const expected: ExpectedControl[] = [
	{ file: 'pages/found.html', outcome: 'pass', reason: 'real id' },
	{ file: 'pages/absent.html', outcome: 'finding', reason: 'missing id' },
];
const report = {
	usage: { model_requests: 0 },
	results: [
		{ file: 'pages/found.html', outcome: 'pass' },
		{ file: 'pages/absent.html', outcome: 'finding' },
	],
};
test('accounts for individual control owners and outcomes', () => {
	requireControls(report, expected);
});
test('rejects equal totals with the same owner repeated', () => {
	const repeated = { file: 'pages/found.html', outcome: 'pass' };
	assert.throws(() => requireControls({ ...report, results: [repeated, repeated] }, expected));
});
test('rejects an anchor false pass despite unchanged total', () => {
	assert.throws(() =>
		requireControls(
			{
				...report,
				results: [
					{ file: 'pages/found.html', outcome: 'pass' },
					{ file: 'pages/absent.html', outcome: 'pass' },
				],
			},
			expected,
		),
	);
});
test('rejects actual model dispatch in a prepared control', () => {
	assert.throws(() => requireControls({ ...report, usage: { model_requests: 1 } }, expected));
});
