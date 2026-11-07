/** O6 semantic report checks against authored fixture expectations. */
import { createHash } from 'node:crypto';

import { historicalDescription } from './rulebook-case-fixtures.ts';
import { descriptions, openings, includedFolder } from './rulebook-filters.ts';
import {
	arrayValue,
	equal,
	objectValue,
	requireCondition,
	textValue,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import { skippedDescription, skippedReportFolder } from './rulebook-report.ts';

export interface CaseDisposition {
	readonly id: string;
	readonly disposition: string;
}
function caseIdentityOrder(left: readonly string[], right: readonly string[]): number {
	return (left[0] ?? '').localeCompare(right[0] ?? '');
}
/** Compare identities as well as totals so dropped/repeated cases cannot hide. */
export function requireCaseDispositions(options: {
	readonly observed: readonly CaseDisposition[];
	readonly expected: Readonly<Record<string, string>>;
}): void {
	equal(
		options.observed.map((entry) => [entry.id, entry.disposition]).toSorted(caseIdentityOrder),
		Object.entries(options.expected).toSorted(caseIdentityOrder),
		'Cases missing, repeated, wrongly scored or wrongly listed',
	);
}
function incrementCount(counts: Record<string, number>, key: string): void {
	counts[key] = (counts[key] ?? 0) + 1;
}
function requireCounts(actual: JsonObject, expected: Readonly<Record<string, number>>): void {
	for (const key of new Set([...Object.keys(actual), ...Object.keys(expected)]))
		equal(actual[key] ?? 0, expected[key] ?? 0, 'Case totals differ');
}
export function requireCasesReport(options: {
	report: JsonObject;
	definitions: readonly JsonObject[];
	expected: Readonly<Record<string, string>>;
	notSelected?: readonly string[];
	expectedErrors?: Readonly<Record<string, string>>;
}): void {
	const { report, definitions, expected } = options;
	const rows = arrayValue(report['cases']).map(objectValue);
	requireCaseDispositions({
		observed: rows.map((row) => ({
			id: textValue(row['id']),
			disposition: textValue(objectValue(row['outcome'])['kind']),
		})),
		expected,
	});
	equal(report['errors'], options.expectedErrors ?? {}, 'Case revision diagnostics differ');
	equal(
		arrayValue(report['not_selected']).map(textValue).toSorted(),
		[...(options.notSelected ?? [])].toSorted(),
		'Selected fixture cases were dropped',
	);
	const total: Record<string, number> = {};
	const byRule: Record<string, Record<string, number>> = {};
	const byFilter: Record<string, Record<string, number>> = {};
	for (const row of rows) {
		const id = textValue(row['id']);
		const definition = definitions.find((item) => item['id'] === id);
		requireCondition(definition, 'Unknown case appeared');
		equal(row['case'], definition, 'Case definition changed in output');
		if (id === 'filter-skipped-must-pass')
			equal(
				row['bars'],
				[{ step: 'scope', bound: 'min_probability', bar: 0.7, observed: 0.5, crossed: false }],
				'Known unsure filter skip lost its crossed bar',
			);
		const outcome = textValue(expected[id]);
		incrementCount(total, outcome);
		const owner = textValue(
			definition['kind'] === 'rule' ? definition['rule'] : definition['filter'],
		);
		const groups = definition['kind'] === 'rule' ? byRule : byFilter;
		const group = groups[owner] ?? {};
		incrementCount(group, outcome);
		groups[owner] = group;
		if (
			['stale', 'no_matching_piece', 'more_than_one_matching_piece', 'not_judged'].includes(outcome)
		) {
			equal(row['result_id'], null, 'Unmatched/stale case was scored against an invented result');
			equal(row['bars'], [], 'Unmatched/stale case invented bar crossings');
			if (outcome === 'not_judged')
				requireCondition(
					textValue(objectValue(row['outcome'])['reason']).length > 0,
					'Not-judged case lost its reason',
				);
		} else
			requireCondition(typeof row['result_id'] === 'number', 'Scored case lost its result link');
		if (typeof row['result_id'] === 'number') {
			const run = arrayValue(report['runs'])
				.map(objectValue)
				.find((item) => item['revision'] === definition['revision']);
			requireCondition(run, 'Case result has no revision run');
			const lint = objectValue(run['report']);
			const linked = [
				...arrayValue(lint['results']),
				...arrayValue(lint['filter_answers']),
				...arrayValue(lint['filter_skips']),
			]
				.map(objectValue)
				.find((item) => item['id'] === row['result_id']);
			requireCondition(linked, 'Case result link names no lint result');
			if (definition['kind'] === 'rule') {
				equal(linked['name'] ?? linked['rule'], definition['rule'], 'Case linked to another rule');
				equal(linked['file'], definition['file'], 'Case linked to another file');
				const expectedOutcome =
					outcome === 'right'
						? definition['expected'] === 'warn'
							? 'finding'
							: 'pass'
						: outcome === 'unsure'
							? 'uncertain'
							: outcome === 'filter_excluded'
								? 'not_applicable'
								: 'incomplete';
				if (linked['why'])
					equal(
						outcome,
						linked['why'] === 'unsure' ? 'unsure' : 'incomplete',
						'Case score differs from its filter skip',
					);
				else
					equal(
						linked['outcome'],
						expectedOutcome,
						'Case score differs from its real lint outcome',
					);
			} else {
				equal(linked['filter'], definition['filter'], 'Case linked to another filter');
				equal(linked['folder'], definition['folder'], 'Case linked to another folder');
				equal(
					linked['state'],
					outcome === 'unsure' ? 'unsure' : 'judged',
					'Filter score differs from real filter outcome',
				);
				equal(
					linked['label'] === definition['expected_label'],
					outcome === 'right',
					'Filter label score differs',
				);
			}
		}
	}
	requireCounts(objectValue(report['counts']), total);
	for (const [key, expectedGroups] of [
		['by_rule', byRule],
		['by_filter', byFilter],
	] as const) {
		const actual = objectValue(report[key]);
		for (const owner of new Set([...Object.keys(actual), ...Object.keys(expectedGroups)]))
			requireCounts(objectValue(actual[owner] ?? {}), expectedGroups[owner] ?? {});
	}
}
export function requireRevisionContext(report: JsonObject, prepared: JsonObject): void {
	const runs = arrayValue(report['runs']).map(objectValue);
	equal(
		runs.map((run) => textValue(run['revision'])).toSorted(),
		[textValue(prepared['historical']), textValue(prepared['current'])].toSorted(),
		'Wrong or duplicated revision runs',
	);
	for (const run of runs) {
		equal(run['resolved_revision'], run['revision'], 'Wrong Git revision resolved');
		const historic = run['revision'] === prepared['historical'];
		const lint = objectValue(run['report']);
		const states = arrayValue(lint['requests'])
			.map(objectValue)
			.map((record) => objectValue(record['request'])['state']);
		const text = JSON.stringify(states);
		for (const sentinel of historic
			? [
					'HISTORICAL_REFERENCE_BODY_SENTINEL',
					'HISTORICAL_EXTERNAL_OPENING_SENTINEL',
					historicalDescription,
				]
			: [
					'CURRENT_REFERENCE_BODY_SENTINEL',
					'CURRENT_EXTERNAL_OPENING_SENTINEL',
					'CURRENT_FILTER_DESCRIPTION_SENTINEL',
				])
			requireCondition(
				text.includes(sentinel),
				'Case read the wrong revision context or missed external loaded file',
			);
		for (const sentinel of historic
			? [
					'CURRENT_REFERENCE_BODY_SENTINEL',
					'CURRENT_EXTERNAL_OPENING_SENTINEL',
					'CURRENT_FILTER_DESCRIPTION_SENTINEL',
				]
			: ['HISTORICAL_REFERENCE_BODY_SENTINEL', 'HISTORICAL_EXTERNAL_OPENING_SENTINEL'])
			requireCondition(!text.includes(sentinel), 'Case context mixed revisions');
		requireCondition(
			text.includes('CURRENT_RULEBOOK_REFERENCE_SENTINEL'),
			'Historical tree did not get current rulebook overlay',
		);
		for (const answer of arrayValue(lint['filter_answers']).map(objectValue)) {
			const folder = textValue(answer['folder']);
			const description =
				folder === skippedReportFolder
					? skippedDescription
					: folder === includedFolder && !historic
						? 'CURRENT_FILTER_DESCRIPTION_SENTINEL.'
						: descriptions.get(folder);
			const opening = folder === skippedReportFolder ? 'Unsure opening.' : openings.get(folder);
			requireCondition(description && opening, 'Unknown hash fixture folder');
			const expected = createHash('sha256')
				.update(JSON.stringify({ description, opening }))
				.digest('hex');
			equal(
				answer['reads_sha256'],
				expected,
				'Filter hash does not identify declared unprojected historical fields',
			);
		}
	}
}
export function requireBars(report: JsonObject, changed: boolean): void {
	const bars = arrayValue(report['bars']).map(objectValue);
	equal(bars.length, 4, 'Recorded bars lost or duplicated');
	equal(
		bars
			.map((bar) => `${textValue(bar['for'])}/${textValue(bar['step'])}/${textValue(bar['bound'])}`)
			.toSorted(),
		[
			'asking_gate/approved/no_at_or_below',
			'asking_gate/approved/yes_at_or_above',
			'category/scope/min_probability',
			'category/style/min_probability',
		],
		'Recorded bar identities differ',
	);
	for (const bar of bars) {
		const affected = bar['for'] === 'asking_gate' && bar['bound'] === 'yes_at_or_above';
		equal(
			bar['state'],
			changed && affected ? 'unassessed' : 'assessed',
			'Changed bar not distinguished from assessed bar',
		);
		if (changed && affected) {
			equal(bar['value'], 0.7, 'Recorded bar changed with current tree');
			equal(bar['current'], 0.8, 'Changed current bar lost');
			requireCondition(textValue(bar['reason']).includes('changed'), 'Unassessed reason lost');
		}
	}
	const rows = arrayValue(report['cases']).map(objectValue);
	for (const [id, observed] of [
		['must-pass', 0.95],
		['must-warn', 0.1],
		['unsure', 0.5],
	] as const) {
		const row = rows.find((item) => item['id'] === id);
		if (!row) continue;
		const crossings = arrayValue(row['bars']).map(objectValue);
		equal(crossings.length, 2, 'Case lost its bar crossings');
		equal(
			crossings.map((crossing) => textValue(crossing['bound'])).toSorted(),
			['no_at_or_below', 'yes_at_or_above'],
			'Case bar comparisons missing or duplicated',
		);
		for (const crossing of crossings) {
			equal(crossing['step'], 'approved', 'Wrong case bar step');
			equal(crossing['observed'], observed, 'Case bar does not name observed probability');
			const bar = crossing['bound'] === 'yes_at_or_above' ? (changed ? 0.8 : 0.7) : 0.3;
			equal(crossing['bar'], bar, 'Case crossed wrong bar');
			equal(
				crossing['crossed'],
				crossing['bound'] === 'yes_at_or_above' ? observed >= bar : observed <= bar,
				'Case bar comparison differs',
			);
		}
	}
}
function withoutLocalIds(row: JsonObject): JsonObject {
	return Object.fromEntries(
		Object.entries(row).filter(([key]) => !['id', 'request_ids'].includes(key)),
	);
}
export function ordinaryProjection(report: JsonObject): JsonObject {
	return {
		results: arrayValue(report['results']).map(objectValue).map(withoutLocalIds),
		filters: arrayValue(report['filter_answers']).map(objectValue).map(withoutLocalIds),
		skips: arrayValue(report['filter_skips']).map(objectValue).map(withoutLocalIds),
	};
}

/** Authored checked paths, intersected with selected case scope, are the independent lint oracle. */
export function selectedFixturePaths(
	definitions: readonly JsonObject[],
	revision: string,
	checkedFiles: readonly string[],
): string[] {
	const selected = definitions.filter((definition) => definition['revision'] === revision);
	return checkedFiles
		.filter((file) =>
			selected.some((definition) =>
				definition['kind'] === 'rule'
					? definition['file'] === file
					: file.startsWith(`${textValue(definition['folder'])}/`),
			),
		)
		.toSorted();
}

export function requireCaseScope(report: JsonObject, expectedPaths: readonly string[]): void {
	for (const row of [...arrayValue(report['results']), ...arrayValue(report['filter_skips'])].map(
		objectValue,
	))
		requireCondition(
			expectedPaths.includes(textValue(row['file'])),
			'Case run judged an unselected file',
		);
	const text = JSON.stringify(report);
	requireCondition(
		!text.includes('UNRELATED_CASE_SCOPE_SENTINEL') && !text.includes('Unrelated folder guidance.'),
		'Case run consumed unrelated folder context',
	);
}

/** Invalid evidence must keep the recorded value while refusing to assess that one bar. */
export function requireInvalidBarSource(report: JsonObject, setFrom: readonly string[]): void {
	const bars = arrayValue(report['bars']).map(objectValue);
	equal(bars.length, 4, 'Invalid provenance dropped bars');
	for (const bar of bars) {
		const affected = bar['for'] === 'asking_gate' && bar['bound'] === 'yes_at_or_above';
		equal(
			bar['state'],
			affected ? 'unassessed' : 'assessed',
			'Invalid bar source was assessed or leaked to other bars',
		);
		if (affected) {
			equal(bar['set_from'], setFrom, 'Invalid source identity changed');
			equal(bar['value'], 0.7, 'Recorded bar changed');
			equal(bar['current'], 0.7, 'Current bar changed');
			requireCondition(textValue(bar['reason']).length > 0, 'Invalid bar source has no reason');
		}
	}
}
