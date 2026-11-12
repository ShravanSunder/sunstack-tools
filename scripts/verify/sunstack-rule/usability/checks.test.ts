import assert from 'node:assert/strict';

import { test } from 'vitest';

import {
	requireScope,
	requireProblemSummary,
	requirePreparedSummary,
	requireCredentialRefusal,
	requireNoSelection,
	requireConfigReason,
	requireHelp,
} from './checks.ts';

test('status-only summary counts diagnostics and rejects the old finding-count contradiction', () => {
	requireProblemSummary('Invalid input: 1 problem\n', 'Invalid input', 1);
	assert.throws(() =>
		requireProblemSummary('Invalid input: 0 error(s), 0 warning(s)\n', 'Invalid input', 1),
	);
	assert.throws(() =>
		requireProblemSummary('elsewhere: 1 problem\nInvalid input: unknown\n', 'Invalid input', 1),
	);
});
test('prepared summary binds count, grammar and actual output path', () => {
	requirePreparedSummary(
		'Prepared 1 request in /outside/one.json: 0 error(s), 0 warning(s)\n',
		1,
		'/outside/one.json',
	);
	assert.throws(() =>
		requirePreparedSummary(
			'Prepared 2 requests in /outside/one.json: 0 error(s), 0 warning(s)\n',
			1,
			'/outside/one.json',
		),
	);
	assert.throws(() =>
		requirePreparedSummary(
			'Prepared 1 request in /other.json: 0 error(s), 0 warning(s)\n',
			1,
			'/outside/one.json',
		),
	);
	requirePreparedSummary(
		'Prepared 0 requests in /outside/none.json: 0 error(s), 0 warning(s)',
		0,
		'/outside/none.json',
	);
});
test('named scope refuses the old early uncommitted label', () => {
	requireScope('Scope: all configured targets\nInvalid input: 1 problem', true);
	assert.throws(() => requireScope('Scope: uncommitted\nInvalid input: 1 problem', true));
});
const refusal = {
	status: { kind: 'invalid' },
	diagnostics: [{ code: 'credential_missing' }],
	usage: { model_requests: 0 },
	results: [],
	requests: [],
	findings: [],
	file_counts: [],
	selections: [],
};
test('credential negative cannot pass on a clean result or partial evidence account', () => {
	requireCredentialRefusal(refusal);
	assert.throws(() => requireCredentialRefusal({ ...refusal, status: { kind: 'clean' } }));
	assert.throws(() => requireCredentialRefusal({ ...refusal, file_counts: [{ file: 'read.md' }] }));
});
test('empty ordinary run is an explicit nothing-selected outcome', () => {
	requireNoSelection({ ...refusal, status: { kind: 'nothing_selected' } });
	assert.throws(() => requireNoSelection(refusal));
});
test('missing custom config names the path and a reason rather than the default basename', () => {
	requireConfigReason('nope.json: config not found', 'nope.json', /not found/);
	assert.throws(() =>
		requireConfigReason(
			'nope.json: Cannot open the selected sunstack.lint.json.',
			'nope.json',
			/not found/,
		),
	);
});
const help =
	'Options:\n  --config <FILE> Config file to use; default: nearest sunstack.lint.json upward.\n  --root <DIR> checked root\nExit codes:\n  0 Clean or nothing selected\n  1 Findings\n  2 Invalid input\n  3 Incomplete\n  4 Cancelled\n  5 Uncertain\n  6 Prepared\n';
test('help binds config meaning to its option and every exit to its actual meaning', () => {
	requireHelp(help);
	assert.throws(() => requireHelp(help.replace('  6 Prepared', '  6 Success')));
	assert.throws(() =>
		requireHelp(
			help
				.replace('Config file to use; default: nearest sunstack.lint.json upward.', 'file')
				.replace('checked root', 'nearest sunstack.lint.json upward config file'),
		),
	);
});

test('prepared output filename must not merely be a prefix of another destination', () => {
	assert.throws(() =>
		requirePreparedSummary(
			'Prepared 1 request in /outside/one.json.other: 0 error(s), 0 warning(s)',
			1,
			'/outside/one.json',
		),
	);
	requirePreparedSummary(
		'Prepared 2 requests in /outside/two requests.json: 0 error(s), 0 warning(s)',
		2,
		'/outside/two requests.json',
	);
});
