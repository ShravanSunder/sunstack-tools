/** O6 real lint cases: revision context, complete accounting and no adoption gate. */
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';
import { isDeepStrictEqual } from 'node:util';

import { requireDebugSignature } from './rulebook-amendment.ts';
import {
	casesOracle,
	casesConfigName,
	prepareCasesFixture,
	historicalDescription,
} from './rulebook-case-fixtures.ts';
import { descriptions, openings, includedFolder } from './rulebook-filters.ts';
import {
	arrayValue,
	equal,
	gitCommand,
	journeyArguments,
	objectValue,
	parseObject,
	readObject,
	requireCondition,
	rulebookName,
	rulesName,
	runSequence,
	textValue,
	writeJson,
	type JsonObject,
	type JourneyArguments,
} from './rulebook-fixture-support.ts';
import {
	declaredTarget,
	responseBody,
	RulebookProcess,
	type CapturedRequest,
	type ControlledReply,
} from './rulebook-process.ts';
import {
	reportReply,
	requireSavedRequest,
	skippedDescription,
	skippedReportFolder,
} from './rulebook-report.ts';

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
function casesReply(body: JsonObject, alwaysUnsure: boolean): ControlledReply {
	const state = objectValue(body['state']);
	if (!('scope' in objectValue(body['questions'])) && alwaysUnsure)
		return { status: 200, body: responseBody(body, 0.5) };
	if (state['description'] === 'CURRENT_FILTER_DESCRIPTION_SENTINEL.')
		return reportReply({ ...body, state: { ...state, description: historicalDescription } });
	return reportReply(body);
}
function caseDefinitions(prepared: JsonObject): JsonObject[] {
	return [...arrayValue(prepared['rule_cases']), ...arrayValue(prepared['filter_cases'])].map(
		objectValue,
	);
}
function caseFile(
	prepared: JsonObject,
	root: string,
	definitions = caseDefinitions(prepared),
): JsonObject {
	return { repository: root, cases: definitions, bars: prepared['bars'] ?? [] };
}
function expectedDispositions(prepared: JsonObject, alwaysUnsure: boolean): Record<string, string> {
	const expected: Record<string, string> = {};
	for (const [id, value] of Object.entries(objectValue(prepared['expected_dispositions'])))
		expected[id] = textValue(value);
	if (alwaysUnsure)
		for (const id of [
			'pass',
			'must-pass',
			'must-warn',
			'unsure',
			'incomplete',
			'historical-reference',
			'current-pass',
			'current-reference',
		])
			expected[id] = 'unsure';
	return expected;
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
	equal(report['errors'], {}, 'Case revision failed before evaluation');
	equal(report['not_selected'], [], 'Selected fixture cases were dropped');
	const total: Record<string, number> = {};
	const byRule: Record<string, Record<string, number>> = {};
	const byFilter: Record<string, Record<string, number>> = {};
	for (const row of rows) {
		const id = textValue(row['id']);
		const definition = definitions.find((item) => item['id'] === id);
		requireCondition(definition, 'Unknown case appeared');
		equal(row['case'], definition, 'Case definition changed in output');
		const outcome = textValue(expected[id]);
		incrementCount(total, outcome);
		const owner = textValue(
			definition['kind'] === 'rule' ? definition['rule'] : definition['filter'],
		);
		const groups = definition['kind'] === 'rule' ? byRule : byFilter;
		const group = groups[owner] ?? {};
		incrementCount(group, outcome);
		groups[owner] = group;
		if (['stale', 'no_matching_piece', 'more_than_one_matching_piece'].includes(outcome))
			equal(row['result_id'], null, 'Unmatched/stale case was scored against an invented result');
		else requireCondition(typeof row['result_id'] === 'number', 'Scored case lost its result link');
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
				equal(linked['outcome'], expectedOutcome, 'Case score differs from its real lint outcome');
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
function requireCaseRequests(
	report: JsonObject,
	captures: readonly CapturedRequest[],
	alwaysUnsure: boolean,
): void {
	const records = arrayValue(report['runs'])
		.map(objectValue)
		.flatMap((run) => arrayValue(objectValue(run['report'])['requests']).map(objectValue));
	equal(records.length, captures.length, 'Case runs lost or invented captured requests');
	const consumed = new Set<number>();
	for (const record of records) {
		equal(record['state'], 'run', 'Fixture request did not dispatch');
		const request = objectValue(record['request']);
		const index = captures.findIndex(
			(capture, position) =>
				!consumed.has(position) &&
				capture.body['model'] === record['requested_model'] &&
				isDeepStrictEqual(capture.body['state'], request['state']) &&
				isDeepStrictEqual(capture.body['questions'], request['questions']),
		);
		requireCondition(index >= 0, 'Saved case request has no actual HTTP request');
		consumed.add(index);
		const captured = captures[index];
		requireCondition(captured, 'Capture missing');
		const outcome = objectValue(record['outcome']);
		const reply = casesReply(captured.body, alwaysUnsure);
		equal(
			outcome['kind'],
			reply.status === 200 ? 'completed' : 'provider_failure',
			'Case request outcome differs from controlled response',
		);
		if (reply.status === 400) {
			equal(outcome['failure_class'], 'provider_rejected_request', 'Case refusal class differs');
			equal(outcome['http_status'], 400, 'Case refusal status differs');
		}
		requireSavedRequest({
			sent: captured.body,
			savedState: request['state'],
			savedQuestions: request['questions'],
			savedAnswers: outcome['answers'],
			requestedModel: record['requested_model'],
			resolvedModel: record['resolved_model'],
			reply,
		});
		const state = objectValue(request['state']);
		for (const field of [
			'expected',
			'expected_label',
			'label_from',
			'set',
			'text_sha256',
			'reads_sha256',
			'lines',
		])
			requireCondition(!(field in state), 'Case metadata leaked into model state');
	}
}
function requireRevisionContext(report: JsonObject, prepared: JsonObject): void {
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
function requireBars(report: JsonObject, changed: boolean): void {
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
function ordinaryProjection(report: JsonObject): JsonObject {
	return {
		results: arrayValue(report['results']).map(objectValue).map(withoutLocalIds),
		filters: arrayValue(report['filter_answers']).map(objectValue).map(withoutLocalIds),
		skips: arrayValue(report['filter_skips']).map(objectValue).map(withoutLocalIds),
	};
}
function prepareOrdinaryCopy(root: string, revision: string, evidence: string): string {
	const archive = path.join(evidence, `${revision}.tar`);
	const destination = path.join(evidence, revision);
	gitCommand(root, ['archive', '--format=tar', '--output', archive, revision]);
	filesystem.mkdirSync(destination);
	const extracted = spawnSync('tar', ['-xf', archive, '-C', destination], {
		encoding: 'utf8',
		timeout: 15000,
	});
	requireCondition(
		extracted.status === 0,
		`Fixture archive extraction failed: ${extracted.stderr}`,
	);
	gitCommand(destination, ['init', '-q']);
	for (const file of [casesConfigName, rulebookName, rulesName, 'evidence/sgconfig.yml'])
		filesystem.copyFileSync(path.join(root, file), path.join(destination, file));
	return destination;
}
export async function runCasesJourney(options: JourneyArguments): Promise<void> {
	const target = options.prepare
		? undefined
		: declaredTarget({
				binary: options.binary ?? '',
				source: options['source-root'],
				commit: options.commit ?? '',
				...(options.sha256 ? { digest: options.sha256 } : {}),
			});
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const root = path.join(evidence, 'fixture');
	const prepared = prepareCasesFixture(root);
	const definitions = caseDefinitions(prepared);
	const input = path.join(evidence, 'cases.json');
	writeJson(path.join(evidence, 'prepared.json'), prepared);
	writeJson(input, caseFile(prepared, root));
	if (options.prepare) {
		process.stdout.write('Prepared case revision fixtures; product unobserved\n');
		return;
	}
	requireCondition(target, 'Case observation needs declared candidate');
	requireDebugSignature(target.binary);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		const currentConfigPath = path.join(root, casesConfigName);
		const currentConfig = readObject(currentConfigPath);
		objectValue(currentConfig['decision_model'])['endpoint'] = observer.endpoint;
		writeJson(currentConfigPath, currentConfig);
		observer.respond = ({ body }) => Promise.resolve(casesReply(body, false));
		const checked = await observer.run({
			root,
			name: 'check',
			arguments: ['lint', '--config', casesConfigName, '--check'],
			expectedExit: 0,
		});
		equal(checked.captures.length, 0, 'Keyless check dispatched');
		const before = await observer.run({
			root,
			name: 'lint-before',
			arguments: ['lint', '--config', casesConfigName, '--all'],
			credential: true,
			expectedExit: 3,
		});
		const positive = path.join(evidence, 'positive-cases.json');
		const positiveIds = [
			'pass',
			'must-pass',
			'must-warn',
			'filter-right',
			'historical-reference',
			'current-pass',
			'current-reference',
		];
		const positives = definitions.filter((item) => positiveIds.includes(textValue(item['id'])));
		writeJson(positive, caseFile(prepared, root, positives));
		const good = await observer.run({
			root,
			name: 'positive-cases',
			arguments: ['lint', '--config', casesConfigName, 'cases', positive, '--set', 'development'],
			credential: true,
			expectedExit: 0,
		});
		const goodReport = parseObject(good.stdout.toString('utf8'));
		const goodExpected: Record<string, string> = {};
		for (const id of positiveIds) goodExpected[id] = 'right';
		requireCasesReport({ report: goodReport, definitions: positives, expected: goodExpected });
		requireBars(goodReport, false);
		const command = ['lint', '--config', casesConfigName, 'cases', input, '--set', 'development'];
		const mixed = await observer.run({
			root,
			name: 'mixed-cases',
			arguments: command,
			credential: true,
			expectedExit: 1,
		});
		const mixedReport = parseObject(mixed.stdout.toString('utf8'));
		requireCasesReport({
			report: mixedReport,
			definitions,
			expected: expectedDispositions(prepared, false),
		});
		requireCaseRequests(mixedReport, mixed.captures, false);
		requireRevisionContext(mixedReport, prepared);
		requireBars(mixedReport, false);
		const runs = arrayValue(mixedReport['runs']).map(objectValue);
		await runSequence(runs, async (run) => {
			const revision = textValue(run['revision']);
			const copied = prepareOrdinaryCopy(root, revision, evidence);
			const result = await observer.report({
				root: copied,
				name: `ordinary-${revision}`,
				arguments: ['lint', '--config', casesConfigName, '--format', 'json', '--all'],
				credential: true,
				expectedExit: 3,
			});
			equal(
				ordinaryProjection(result.report),
				ordinaryProjection(objectValue(run['report'])),
				'Lint and case run differ on the same text/context',
			);
		});
		const rulesPath = path.join(root, rulesName);
		const originalRules = filesystem.readFileSync(rulesPath);
		const rules = readObject(rulesPath);
		const askingRule = objectValue(rules['K002']);
		const decisionSteps = objectValue(objectValue(askingRule['decision'])['steps']);
		objectValue(decisionSteps['approved'])['yes_at_or_above'] = 0.8;
		writeJson(rulesPath, rules);
		try {
			const changed = await observer.run({
				root,
				name: 'changed-bar',
				arguments: command,
				credential: true,
				expectedExit: 1,
			});
			const report = parseObject(changed.stdout.toString('utf8'));
			requireCasesReport({ report, definitions, expected: expectedDispositions(prepared, false) });
			requireBars(report, true);
		} finally {
			filesystem.writeFileSync(rulesPath, originalRules);
		}
		observer.respond = ({ body }) => Promise.resolve(casesReply(body, true));
		const unsure = await observer.run({
			root,
			name: 'always-unsure',
			arguments: command,
			credential: true,
			expectedExit: 1,
		});
		const unsureReport = parseObject(unsure.stdout.toString('utf8'));
		requireCasesReport({
			report: unsureReport,
			definitions,
			expected: expectedDispositions(prepared, true),
		});
		requireCaseRequests(unsureReport, unsure.captures, true);
		observer.respond = ({ body }) => Promise.resolve(casesReply(body, false));
		const after = await observer.run({
			root,
			name: 'lint-after',
			arguments: ['lint', '--config', casesConfigName, '--all'],
			credential: true,
			expectedExit: 3,
		});
		equal(after.stdout, before.stdout, 'Case runs changed ordinary lint stdout bytes');
		equal(after.stderr, before.stderr, 'Case runs changed ordinary lint stderr bytes');
	} finally {
		await observer.close();
	}
	writeJson(path.join(evidence, 'pass.json'), {
		product_observed: true,
		oracle: casesOracle,
		commit: target.commit,
		binary_sha256: target.digest,
		unverified: ['real judgment quality', 'ai-tools held-out run', 'release publication'],
	});
	process.stdout.write('Revision cases, bars and lint independence journey passed\n');
}
if (process.argv[1] === import.meta.filename) {
	const options = journeyArguments(
		'O6 revision-bound cases, exact accounting, changed bars, always-unsure and lint independence through the real binary.',
	);
	if (options) await runCasesJourney(options);
}
