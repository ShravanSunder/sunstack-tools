import { expect, test } from 'vitest';

import {
	requireCaseDispositions,
	requireCasesReport,
	type CaseDisposition,
} from './rulebook-cases.ts';
import { arrayValue, objectValue } from './rulebook-fixture-support.ts';

const expected = {
	warn: 'right',
	pass: 'right',
	stale: 'stale',
	ambiguous: 'more_than_one_matching_piece',
	excluded: 'filter_excluded',
};
function dispositions(): CaseDisposition[] {
	return [
		{ id: 'warn', disposition: 'right' },
		{ id: 'pass', disposition: 'right' },
		{ id: 'stale', disposition: 'stale' },
		{ id: 'ambiguous', disposition: 'more_than_one_matching_piece' },
		{ id: 'excluded', disposition: 'filter_excluded' },
	];
}
test('case accounting accepts allowed result order while retaining every identity', () => {
	expect(() =>
		requireCaseDispositions({ observed: dispositions().toReversed(), expected }),
	).not.toThrow();
});
test.each([
	['case silently dropped', (rows: CaseDisposition[]) => rows.pop()],
	[
		'duplicate hiding a dropped case in equal totals',
		(rows: CaseDisposition[]) => {
			rows.pop();
			rows.push({ id: 'warn', disposition: 'right' });
		},
	],
	[
		'stale case scored',
		(rows: CaseDisposition[]) => {
			rows[2] = { id: 'stale', disposition: 'right' };
		},
	],
	[
		'excluded must-pass counted as success',
		(rows: CaseDisposition[]) => {
			rows[4] = { id: 'excluded', disposition: 'right' };
		},
	],
])('case accounting rejects %s', (_name, mutate) => {
	const rows = dispositions();
	mutate(rows);
	expect(() => requireCaseDispositions({ observed: rows, expected })).toThrow();
});

function publicCaseFixture(): Parameters<typeof requireCasesReport>[0] {
	const common = { revision: 'a'.repeat(40), set: 'development', label_from: 'blind-agents' };
	const good = {
		...common,
		id: 'good',
		kind: 'rule',
		rule: 'fixture-rule',
		file: 'skills/a.md',
		lines: [1, 1],
		text_sha256: 'b'.repeat(64),
		expected: 'pass',
	};
	const stale = { ...good, id: 'stale', text_sha256: '0'.repeat(64) };
	const filter = {
		...common,
		id: 'filter',
		kind: 'filter',
		filter: 'kind',
		folder: 'skills/a',
		reads_sha256: 'c'.repeat(64),
		expected_label: 'eligible',
	};
	return {
		definitions: [good, stale, filter],
		expected: { good: 'right', stale: 'stale', filter: 'wrong_label' },
		report: {
			errors: {},
			not_selected: [],
			cases: [
				{ id: 'good', case: good, outcome: { kind: 'right' }, result_id: 1, bars: [] },
				{ id: 'stale', case: stale, outcome: { kind: 'stale' }, result_id: null, bars: [] },
				{ id: 'filter', case: filter, outcome: { kind: 'wrong_label' }, result_id: 2, bars: [] },
			],
			counts: { right: 1, stale: 1, wrong_label: 1 },
			by_rule: { 'fixture-rule': { right: 1, stale: 1 } },
			by_filter: { kind: { wrong_label: 1 } },
			runs: [
				{
					revision: common.revision,
					report: {
						results: [{ id: 1, name: 'fixture-rule', file: 'skills/a.md', outcome: 'pass' }],
						filter_answers: [
							{ id: 2, filter: 'kind', folder: 'skills/a', state: 'judged', label: 'blocked' },
						],
						filter_skips: [],
					},
				},
			],
		},
	};
}
test('public case report accounts by rule and filter and retains the original definitions', () => {
	expect(() => requireCasesReport(publicCaseFixture())).not.toThrow();
});
test.each([
	[
		'wrong per-filter count despite correct global totals',
		(fixture: Parameters<typeof requireCasesReport>[0]) => {
			fixture.report['by_filter'] = { kind: { wrong_label: 2 } };
		},
	],
	[
		'case labels replaced with the rule answer',
		(fixture: Parameters<typeof requireCasesReport>[0]) => {
			objectValue(arrayValue(fixture.report['cases'])[2])['case'] = {
				...objectValue(objectValue(arrayValue(fixture.report['cases'])[2])['case']),
				expected_label: 'blocked',
			};
		},
	],
	[
		'stale case attached to an invented scored result',
		(fixture: Parameters<typeof requireCasesReport>[0]) => {
			objectValue(arrayValue(fixture.report['cases'])[1])['result_id'] = 99;
		},
	],
	[
		'unscored disposition missing from accounting',
		(fixture: Parameters<typeof requireCasesReport>[0]) => {
			fixture.report['counts'] = { right: 1, wrong_label: 1 };
		},
	],
])('public case report rejects %s', (_name, mutate) => {
	const fixture = publicCaseFixture();
	mutate(fixture);
	expect(() => requireCasesReport(fixture)).toThrow();
});
