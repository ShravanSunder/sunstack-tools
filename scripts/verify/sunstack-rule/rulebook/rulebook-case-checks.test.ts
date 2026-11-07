import { expect, test } from 'vitest';

import {
	requireCaseDispositions,
	requireCasesReport,
	type CaseDisposition,
	selectedFixturePaths,
	requireCaseScope,
	requireInvalidBarSource,
} from './rulebook-case-checks.ts';
import {
	arrayValue,
	objectValue,
	requireCondition,
	type JsonObject,
} from './rulebook-fixture-support.ts';

const expected = {
	warn: 'right',
	pass: 'right',
	stale: 'stale',
	ambiguous: 'more_than_one_matching_piece',
	excluded: 'filter_excluded',
	'spanning-range': 'no_matching_piece',
};
function dispositions(): CaseDisposition[] {
	return [
		{ id: 'warn', disposition: 'right' },
		{ id: 'pass', disposition: 'right' },
		{ id: 'stale', disposition: 'stale' },
		{ id: 'ambiguous', disposition: 'more_than_one_matching_piece' },
		{ id: 'excluded', disposition: 'filter_excluded' },
		{ id: 'spanning-range', disposition: 'no_matching_piece' },
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
		'spanning range mistaken for exact-duplicate ambiguity',
		(rows: CaseDisposition[]) => {
			rows[5] = { id: 'spanning-range', disposition: 'more_than_one_matching_piece' };
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
function notJudgedCaseFixture(): Parameters<typeof requireCasesReport>[0] {
	const fixture = publicCaseFixture();
	const row = objectValue(arrayValue(fixture.report['cases'])[0]);
	row['outcome'] = { kind: 'not_judged', reason: 'selected path is a symlink' };
	row['result_id'] = null;
	fixture.expected = { ...fixture.expected, good: 'not_judged' };
	fixture.report['counts'] = { not_judged: 1, stale: 1, wrong_label: 1 };
	fixture.report['by_rule'] = { 'fixture-rule': { not_judged: 1, stale: 1 } };
	return fixture;
}
test('unjudged alias stays listed with a reason outside scored outcomes', () => {
	expect(() => requireCasesReport(notJudgedCaseFixture())).not.toThrow();
});
test.each(['reason', 'result_id', 'bars', 'counts'])(
	'unjudged alias checker refuses invented or lost %s',
	(field) => {
		const fixture = notJudgedCaseFixture();
		const row = objectValue(arrayValue(fixture.report['cases'])[0]);
		if (field === 'reason') objectValue(row['outcome'])['reason'] = '';
		else if (field === 'result_id') row['result_id'] = 1;
		else if (field === 'bars') row['bars'] = [{ step: 'invented' }];
		else fixture.report['counts'] = { right: 1, stale: 1, wrong_label: 1 };
		expect(() => requireCasesReport(fixture)).toThrow();
	},
);
function skippedRuleCaseFixture(): Parameters<typeof requireCasesReport>[0] {
	const fixture = publicCaseFixture();
	const first = fixture.definitions[0];
	requireCondition(first, 'Missing source case fixture');
	const skipped = {
		...first,
		id: 'filter-skipped-must-pass',
		file: 'skills/skip/notes/one.md',
		lines: [5, 5],
	};
	fixture.definitions = [...fixture.definitions, skipped];
	fixture.expected = { ...fixture.expected, 'filter-skipped-must-pass': 'unsure' };
	arrayValue(fixture.report['cases']).push({
		id: skipped.id,
		case: skipped,
		outcome: { kind: 'unsure' },
		result_id: 3,
		bars: [{ step: 'scope', bound: 'min_probability', bar: 0.7, observed: 0.5, crossed: false }],
	});
	objectValue(fixture.report['counts'])['unsure'] = 1;
	objectValue(objectValue(fixture.report['by_rule'])['fixture-rule'])['unsure'] = 1;
	const lint = objectValue(objectValue(arrayValue(fixture.report['runs'])[0])['report']);
	arrayValue(lint['filter_skips']).push({
		id: 3,
		rule: 'fixture-rule',
		file: skipped.file,
		filter: 'category',
		folder: 'skills/skip',
		why: 'unsure',
		path: ['scope'],
		answers: {
			scope: {
				type: 'choice',
				selected_label: 'reusable',
				confidence: 0.99,
				probabilities: { reusable: 0.5, product: 0.25, other: 0.25 },
			},
		},
		locations: [{ file: skipped.file, first_line: 5, last_line: 5 }],
	});
	return fixture;
}
test('public case report accepts a filter-skipped rule with its known Choice bar', () => {
	expect(() => requireCasesReport(skippedRuleCaseFixture())).not.toThrow();
});
test('public case report rejects missing skip bars despite correct outcome counts and link', () => {
	const fixture = skippedRuleCaseFixture();
	const skipped = arrayValue(fixture.report['cases'])
		.map(objectValue)
		.find((row) => row['id'] === 'filter-skipped-must-pass');
	requireCondition(skipped, 'Missing skipped rule case');
	skipped['bars'] = [];
	expect(() => requireCasesReport(fixture)).toThrow(/skip.*bar|bar.*skip/i);
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

test('selected cases include checked folder members and rule paths only at their revision', () => {
	const paths = selectedFixturePaths(
		[
			{ kind: 'rule', file: 'a/one.md', revision: 'old' },
			{ kind: 'filter', folder: 'b', revision: 'old' },
			{ kind: 'rule', file: 'c/current.md', revision: 'new' },
		],
		'old',
		['a/one.md', 'a/unselected.md', 'b/one.md', 'b/two.md', 'b-extra/no.md', 'c/current.md'],
	);
	expect(paths).toEqual(['a/one.md', 'b/one.md', 'b/two.md']);
});
test('case scope rejects an unselected result even with no unrelated sentinel', () => {
	expect(() =>
		requireCaseScope({ results: [{ file: 'other.md' }], filter_skips: [] }, ['selected.md']),
	).toThrow();
});
test('case scope rejects an unrelated filter even with no rule results', () => {
	expect(() =>
		requireCaseScope(
			{
				results: [],
				filter_skips: [],
				filter_answers: [{ description: 'Unrelated folder guidance.' }],
			},
			['selected.md'],
		),
	).toThrow();
});
function invalidSourceReport(): JsonObject {
	return {
		bars: [
			{
				for: 'asking_gate',
				step: 'approved',
				bound: 'yes_at_or_above',
				value: 0.7,
				current: 0.7,
				set_from: ['held-out'],
				state: 'unassessed',
				reason: 'held-out evidence',
			},
			...['no_at_or_below', 'scope', 'style'].map((bound) => ({ bound, state: 'assessed' })),
		],
	};
}
test('bar provenance checker accepts a reasoned refusal at the unchanged threshold', () => {
	expect(() => requireInvalidBarSource(invalidSourceReport(), ['held-out'])).not.toThrow();
});
test.each(['state', 'reason', 'current', 'set_from'])(
	'bar provenance checker rejects a changed %s despite correct result counts',
	(field) => {
		const report = invalidSourceReport();
		const bar = objectValue(arrayValue(report['bars'])[0]);
		bar[field] =
			field === 'state'
				? 'assessed'
				: field === 'current'
					? 0.8
					: field === 'reason'
						? ''
						: ['different'];
		expect(() => requireInvalidBarSource(report, ['held-out'])).toThrow();
	},
);
