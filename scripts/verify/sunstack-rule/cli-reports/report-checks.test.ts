import assert from 'node:assert/strict';

import { test } from 'vitest';

import {
	requireAmbiguity,
	requireCaseDetail,
	requireJsonRefusal,
	requireLintDetail,
	requireSarifRefusal,
} from './report-checks.ts';
import {
	candidateForCase,
	candidatesFor,
	renameSavedCase,
	retainNumericResultRevision,
	type SavedCandidate,
} from './report-inputs.ts';

const savedResult = { id: 27, file: 'right.md', outcome: 'pass', request_ids: [9] };
const request = {
	id: 9,
	request: { state: { body: 'original' }, questions: {} },
	answers: { probability: 0.95 },
};
const candidate: SavedCandidate = {
	revision: 'first',
	caseIds: ['known'],
	result: savedResult,
	report: {
		requests: [request],
		rulebook_files: [],
		piece_evidence: { '27': { original: 'raw' } },
		findings: [],
	},
};
const detail = {
	result: savedResult,
	finding: null,
	requests: [request],
	evidence: { original: 'raw' },
	rulebook_files: [],
	rewalkable: true,
	gap: null,
};
test('keeps same numeric IDs from separate revisions, including an unlinked result', () => {
	const report = {
		cases: [{ id: 'known', case: { revision: 'first' }, result_id: 27 }],
		runs: [
			{
				revision: 'first',
				report: { results: [savedResult], filter_answers: [], filter_skips: [] },
			},
			{
				revision: 'second',
				report: {
					results: [{ ...savedResult, file: 'wrong.md' }],
					filter_answers: [],
					filter_skips: [],
				},
			},
		],
	};
	assert.deepEqual(
		candidatesFor(report, 27).map((entry) => [entry.revision, entry.caseIds, entry.result['file']]),
		[
			['first', ['known'], 'right.md'],
			['second', [], 'wrong.md'],
		],
	);
});
test('accepts exact saved detail payload', () => {
	requireLintDetail(detail, candidate);
});
test('case identity disambiguates a numeric result that exists in two revisions', () => {
	const report = {
		cases: [{ id: 'known', case: { revision: 'first' }, result_id: 27 }],
		runs: [
			{
				revision: 'first',
				report: { results: [savedResult], filter_answers: [], filter_skips: [] },
			},
			{
				revision: 'second',
				report: {
					results: [{ ...savedResult, file: 'wrong.md' }],
					filter_answers: [],
					filter_skips: [],
				},
			},
		],
	};
	assert.equal(candidateForCase(report, 'known').result['file'], 'right.md');
});
test('case wrapper cannot mislabel the revision even with the correct inner payload', () => {
	const expectedCase = { id: 'known', case: { revision: 'first' }, result_id: 27 };
	assert.throws(() =>
		requireCaseDetail({ case: expectedCase, revision: 'second', detail }, candidate, expectedCase),
	);
});
test('ambiguity requires the unlinked candidate as well as the linked one', () => {
	const choices = [candidate, { ...candidate, revision: 'second', caseIds: [] }];
	requireAmbiguity(
		'cases report ambiguous result 27: first known result 27; second no case id result 27',
		27,
		choices,
	);
	assert.throws(() =>
		requireAmbiguity('cases report ambiguous result 27: first known result 27', 27, choices),
	);
});
test('rejects a different owner with the same ID and pass outcome', () => {
	assert.throws(() =>
		requireLintDetail({ ...detail, result: { ...savedResult, file: 'wrong.md' } }, candidate),
	);
});
test('rejects missing saved requests instead of checking result counts alone', () => {
	assert.throws(() => requireLintDetail({ ...detail, requests: [] }, candidate));
});
const refusal = { path: 'assets/offending.bin', reason: /UnsupportedEncoding/ };
test('accepts a structured JSON path separate from the config location', () => {
	requireJsonRefusal(
		{
			diagnostics: [
				{
					file: 'sunstack.lint.json',
					path: 'assets/offending.bin',
					message: 'UnsupportedEncoding',
				},
			],
		},
		'/fixture',
		refusal,
	);
});
test('rejects a path present only in human message text', () => {
	assert.throws(() =>
		requireJsonRefusal(
			{
				diagnostics: [
					{ file: 'sunstack.lint.json', message: 'assets/offending.bin UnsupportedEncoding' },
				],
			},
			'/fixture',
			refusal,
		),
	);
});
test('rejects a limit stop with the wrong configured value', () => {
	assert.throws(() =>
		requireJsonRefusal(
			{ diagnostics: [{ path: 'docs/checked.md', message: 'max_file_bytes 40960' }] },
			'/fixture',
			{
				path: 'docs/checked.md',
				reason: /limit|file_bytes/,
				limit: { name: 'file_bytes', value: 4096 },
			},
		),
	);
});
test('rejects another limit name even when its value matches', () => {
	assert.throws(() =>
		requireJsonRefusal(
			{ diagnostics: [{ path: 'docs/checked.md', message: 'TotalBytes limit 4096' }] },
			'/fixture',
			{ path: 'docs/checked.md', reason: /limit/, limit: { name: 'file_bytes', value: 4096 } },
		),
	);
});
test('accepts the configured limit name/value on the same diagnostic as its path', () => {
	requireJsonRefusal(
		{
			diagnostics: [{ path: 'docs/checked.md', message: 'change scope limit max_file_bytes=4096' }],
		},
		'/fixture',
		{ path: 'docs/checked.md', reason: /limit/, limit: { name: 'file_bytes', value: 4096 } },
	);
});
test('does not count a version number in a path as the configured limit value', () => {
	assert.throws(() =>
		requireJsonRefusal(
			{
				diagnostics: [
					{
						path: 'assets/offending.bin',
						message: 'Files limit reached at /private/tmp/engine-0.11.1/assets/offending.bin',
					},
				],
			},
			'/fixture',
			{ path: 'assets/offending.bin', reason: /limit/, limit: { name: 'files', value: 1 } },
		),
	);
});
test('reads SARIF diagnostic notifications even when no findings exist', () => {
	requireSarifRefusal(
		{
			runs: [
				{
					results: [],
					invocations: [
						{
							toolExecutionNotifications: [
								{
									message: { text: 'UnsupportedEncoding' },
									locations: [
										{ physicalLocation: { artifactLocation: { uri: 'assets/offending.bin' } } },
									],
								},
							],
						},
					],
				},
			],
		},
		'/fixture',
		refusal,
	);
});
test('rejects SARIF with the path only in its message', () => {
	assert.throws(() =>
		requireSarifRefusal(
			{
				runs: [
					{
						results: [],
						invocations: [
							{
								toolExecutionNotifications: [
									{ message: { text: 'assets/offending.bin UnsupportedEncoding' } },
								],
							},
						],
					},
				],
			},
			'/fixture',
			refusal,
		),
	);
});

test('numeric case renaming preserves saved runs and the original input', () => {
	const report = {
		cases: [{ id: 'known', case: { id: 'known', revision: 'first' }, result_id: 27 }],
		runs: [
			{
				revision: 'first',
				report: { results: [savedResult], filter_answers: [], filter_skips: [] },
			},
		],
	};
	const renamed = renameSavedCase(report, 'known', '27');
	assert.deepEqual(renamed['runs'], report.runs);
	assert.deepEqual(report.cases[0], {
		id: 'known',
		case: { id: 'known', revision: 'first' },
		result_id: 27,
	});
	assert.deepEqual(renamed['cases'], [
		{ id: '27', case: { id: '27', revision: 'first' }, result_id: 27 },
	]);
});
test('unique numeric control keeps its own result and unrelated rows while dropping the other collision', () => {
	const report = {
		cases: [{ id: '27', case: { id: '27', revision: 'first' }, result_id: 27 }],
		runs: [
			{
				revision: 'first',
				report: { results: [savedResult], filter_answers: [], filter_skips: [] },
			},
			{
				revision: 'second',
				report: {
					results: [
						{ ...savedResult, file: 'wrong.md' },
						{ ...savedResult, id: 41 },
					],
					filter_answers: [],
					filter_skips: [],
				},
			},
		],
	};
	const unique = retainNumericResultRevision(report, 27, 'first');
	assert.equal(candidatesFor(unique, 27).length, 1);
	assert.equal(candidatesFor(unique, 41).length, 1);
	assert.equal(candidateForCase(unique, '27').revision, 'first');
	assert.equal(candidatesFor(report, 27).length, 2);
});
test('numeric case ambiguity must retain its different own result ID', () => {
	const choices = [
		candidate,
		{ ...candidate, result: { ...savedResult, id: 41 }, caseIds: ['27'] },
	];
	assert.throws(
		() =>
			requireAmbiguity(
				'cases report ambiguous result 27: first known result 27; case id 27 result 27',
				27,
				choices,
			),
		/Candidate result ID omitted/,
	);
	requireAmbiguity(
		'cases report ambiguous result 27: first known result 27; case id 27 result 41',
		27,
		choices,
	);
});
