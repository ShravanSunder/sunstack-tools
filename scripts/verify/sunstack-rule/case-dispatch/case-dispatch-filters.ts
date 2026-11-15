/** Standalone filter and gated-rule dependency controls using the established history fixture. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import { jsonSlice } from '../prepare/raw-json.ts';
import {
	casesConfigName,
	prepareCasesFixture,
	unrelatedDescription,
	unrelatedFolder,
} from '../rulebook/rulebook-case-fixtures.ts';
import { includedFolder } from '../rulebook/rulebook-filters.ts';
import {
	arrayValue,
	objectValue,
	parseObject,
	textValue,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import { responseBody, type RulebookProcess } from '../rulebook/rulebook-process.ts';
import { reportReply } from '../rulebook/rulebook-report.ts';
import {
	attributeControlledCaptures,
	requireCaseAccounting,
	requireCaseRunStatus,
	requireUnassessedBars,
} from './case-dispatch-checks.ts';

function requireEligibleFilter(report: ReturnType<typeof parseObject>, folder: string): void {
	const runs = arrayValue(report['runs']).map(objectValue);
	assert.equal(runs.length, 1);
	const inner = objectValue(runs[0]?.['report']);
	const answers = arrayValue(inner['filter_answers']).map(objectValue);
	assert.equal(answers.length, 1, 'Expected one judged filter answer');
	assert.equal(answers[0]?.['filter'], 'category');
	assert.equal(answers[0]?.['folder'], folder);
	assert.equal(answers[0]?.['state'], 'judged');
	assert.equal(answers[0]?.['label'], 'eligible');
}

export async function observeFilterDispatch(options: {
	readonly observer: RulebookProcess;
	readonly evidence: string;
}): Promise<void> {
	const root = path.join(options.evidence, 'filter-cases-fixture');
	const fixture = prepareCasesFixture(root);
	const rules = arrayValue(fixture['rule_cases']).map(objectValue);
	const filters = arrayValue(fixture['filter_cases']).map(objectValue);
	const selectedRule = rules.find((row) => row['id'] === 'pass');
	const incompleteRule = rules.find((row) => row['id'] === 'incomplete');
	const selectedFilter = filters.find((row) => row['id'] === 'filter-right');
	assert.ok(selectedRule && selectedFilter && incompleteRule);
	const configFile = path.join(root, casesConfigName);
	const config = parseObject(filesystem.readFileSync(configFile, 'utf8'));
	objectValue(config['decision_model'])['endpoint'] = options.observer.endpoint;
	writeJson(configFile, config);
	const casesPath = path.join(options.evidence, 'filter-cases.json');
	const unrelated = {
		id: 'unrelated-filter',
		kind: 'filter',
		filter: 'category',
		folder: unrelatedFolder,
		reads_sha256: createHash('sha256')
			.update(JSON.stringify({ description: unrelatedDescription, opening: 'Unrelated opening.' }))
			.digest('hex'),
		revision: selectedFilter['revision'],
		expected_label: 'eligible',
		set: 'development',
		label_from: 'blind-agents',
	};
	const stale = { ...selectedFilter, id: 'filter-stale', reads_sha256: '0'.repeat(64) };
	writeJson(casesPath, {
		repository: root,
		cases: [selectedRule, selectedFilter, unrelated, stale, incompleteRule],
		bars: [],
	});
	const history = textValue(selectedRule['revision']);
	assert.equal(history, textValue(selectedFilter['revision']));
	const saved = path.join(options.evidence, 'filter-standalone.json');
	const result = await options.observer.run({
		root,
		name: 'filter-standalone',
		arguments: [
			'lint',
			'--config',
			casesConfigName,
			'cases',
			casesPath,
			'--set',
			'development',
			'--case',
			'filter-right',
			'--prepare',
			saved,
		],
		expectedExit: 6,
	});
	assert.equal(result.captures.length, 0);
	const raw = filesystem.readFileSync(saved, 'utf8');
	const report = parseObject(raw);
	assert.deepEqual(report['not_selected'], [
		'pass',
		'unrelated-filter',
		'filter-stale',
		'incomplete',
	]);
	requireCaseAccounting({
		selectedIds: ['filter-right'],
		dispositions: arrayValue(report['cases'])
			.map(objectValue)
			.map((row) => ({
				id: textValue(row['id']),
				kind: textValue(objectValue(row['outcome'])['kind']),
			})),
		expectedKinds: new Map([['filter-right', 'prepared']]),
	});
	assert.equal(arrayValue(report['runs']).length, 1);
	const nested = parseObject(jsonSlice(raw, ['runs', 0, 'report']));
	const owners = arrayValue(nested['requests'])
		.map(objectValue)
		.map((request) => objectValue(request['owner']));
	assert.deepEqual(
		owners,
		[{ kind: 'filter', filter: 'category', folder: includedFolder }],
		'Standalone filter acquired unrelated execution owners',
	);
	const selectedRun = path.join(options.evidence, 'filter-selected-run.json');
	options.observer.reply = (body) => responseBody(body, 0.95, 'reusable');
	const run = await options.observer.run({
		root,
		name: 'filter-standalone-run',
		arguments: [
			'lint',
			'--config',
			casesConfigName,
			'--report',
			selectedRun,
			'cases',
			casesPath,
			'--set',
			'development',
			'--case',
			'filter-right',
		],
		credential: true,
		expectedExit: 1,
	});
	const runRaw = filesystem.readFileSync(selectedRun, 'utf8');
	const standaloneReport = parseObject(runRaw);
	requireUnassessedBars(standaloneReport);
	requireCaseRunStatus(standaloneReport, 'nothing_selected', 0);
	requireEligibleFilter(standaloneReport, includedFolder);
	requireCaseAccounting({
		selectedIds: ['filter-right'],
		dispositions: arrayValue(standaloneReport['cases'])
			.map(objectValue)
			.map((row) => ({
				id: textValue(row['id']),
				kind: textValue(objectValue(row['outcome'])['kind']),
			})),
		expectedKinds: new Map([['filter-right', 'right']]),
	});
	const captures = attributeControlledCaptures(
		jsonSlice(runRaw, ['runs', 0, 'report']),
		run.captures.map((entry) => entry.raw.toString('utf8')),
	);
	assert.deepEqual(
		captures.map((entry) => entry.owner),
		[JSON.stringify(['filter', 'category', includedFolder])],
		'Standalone filter did not dispatch exactly its selected owner',
	);
	const gatedReport = path.join(options.evidence, 'filter-gated-rule-run.json');
	const gated = await options.observer.run({
		root,
		name: 'filter-gated-rule-run',
		arguments: [
			'lint',
			'--config',
			casesConfigName,
			'--report',
			gatedReport,
			'cases',
			casesPath,
			'--set',
			'development',
			'--case',
			'pass',
		],
		credential: true,
		expectedExit: 1,
	});
	const gatedRaw = filesystem.readFileSync(gatedReport, 'utf8');
	const gatedCases = parseObject(gatedRaw);
	requireUnassessedBars(gatedCases);
	requireCaseRunStatus(gatedCases, 'clean', 1);
	requireEligibleFilter(gatedCases, includedFolder);
	requireCaseAccounting({
		selectedIds: ['pass'],
		dispositions: arrayValue(gatedCases['cases'])
			.map(objectValue)
			.map((row) => ({
				id: textValue(row['id']),
				kind: textValue(objectValue(row['outcome'])['kind']),
			})),
		expectedKinds: new Map([['pass', 'right']]),
	});
	const gatedCaptures = attributeControlledCaptures(
		jsonSlice(gatedRaw, ['runs', 0, 'report']),
		gated.captures.map((entry) => entry.raw.toString('utf8')),
	);
	const lines = arrayValue(selectedRule['lines']);
	const firstLine = lines[0];
	const lastLine = lines[1];
	assert.ok(typeof firstLine === 'number' && typeof lastLine === 'number');
	const gatedRuleOwner = JSON.stringify([
		'rule',
		textValue(selectedRule['rule']),
		textValue(selectedRule['file']),
		String(firstLine),
		String(lastLine),
	]);
	const selectedFilterOwner = JSON.stringify(['filter', 'category', includedFolder]);
	assert.deepEqual(
		gatedCaptures.map((entry) => entry.owner).toSorted(),
		[selectedFilterOwner, gatedRuleOwner].toSorted(),
		'Gated selected case did not dispatch exactly its dependency and rule',
	);
	const standaloneBody = captures.find((entry) => entry.owner === selectedFilterOwner)?.raw;
	const dependencyBody = gatedCaptures.find((entry) => entry.owner === selectedFilterOwner)?.raw;
	assert.ok(standaloneBody && dependencyBody, 'Missing standalone or dependency filter HTTP');
	assert.equal(
		dependencyBody,
		standaloneBody,
		'Dependency filter body differs from standalone selection',
	);
	const unrelatedReport = path.join(options.evidence, 'filter-unrelated-positive-run.json');
	const positive = await options.observer.run({
		root,
		name: 'filter-unrelated-positive-run',
		arguments: [
			'lint',
			'--config',
			casesConfigName,
			'--report',
			unrelatedReport,
			'cases',
			casesPath,
			'--set',
			'development',
			'--case',
			'unrelated-filter',
		],
		credential: true,
		expectedExit: 1,
	});
	const unrelatedRaw = filesystem.readFileSync(unrelatedReport, 'utf8');
	const unrelatedCases = parseObject(unrelatedRaw);
	requireUnassessedBars(unrelatedCases);
	requireCaseRunStatus(unrelatedCases, 'nothing_selected', 0);
	requireEligibleFilter(unrelatedCases, unrelatedFolder);
	requireCaseAccounting({
		selectedIds: ['unrelated-filter'],
		dispositions: arrayValue(unrelatedCases['cases'])
			.map(objectValue)
			.map((row) => ({
				id: textValue(row['id']),
				kind: textValue(objectValue(row['outcome'])['kind']),
			})),
		expectedKinds: new Map([['unrelated-filter', 'right']]),
	});
	const positiveCaptures = attributeControlledCaptures(
		jsonSlice(unrelatedRaw, ['runs', 0, 'report']),
		positive.captures.map((entry) => entry.raw.toString('utf8')),
	);
	assert.deepEqual(
		positiveCaptures.map((entry) => entry.owner),
		[JSON.stringify(['filter', 'category', unrelatedFolder])],
		'Unrelated folder has no independently selected HTTP witness',
	);
	const staleReport = path.join(options.evidence, 'filter-stale-run.json');
	const staleRun = await options.observer.run({
		root,
		name: 'filter-stale-run',
		arguments: [
			'lint',
			'--config',
			casesConfigName,
			'--report',
			staleReport,
			'cases',
			casesPath,
			'--set',
			'development',
			'--case',
			'filter-stale',
		],
		credential: true,
		expectedExit: 1,
	});
	assert.equal(staleRun.captures.length, 0, 'Stale filter dispatched a request');
	const staleCases = parseObject(filesystem.readFileSync(staleReport, 'utf8'));
	requireUnassessedBars(staleCases);
	const staleRows = arrayValue(staleCases['cases']).map(objectValue);
	requireCaseAccounting({
		selectedIds: ['filter-stale'],
		dispositions: staleRows.map((row) => ({
			id: textValue(row['id']),
			kind: textValue(objectValue(row['outcome'])['kind']),
		})),
		expectedKinds: new Map([['filter-stale', 'stale']]),
	});
	const incompleteReport = path.join(options.evidence, 'case-incomplete-run.json');
	const previousResponder = options.observer.respond;
	let incompleteRun: Awaited<ReturnType<RulebookProcess['run']>>;
	try {
		options.observer.respond = ({ body }) => Promise.resolve(reportReply(body));
		incompleteRun = await options.observer.run({
			root,
			name: 'case-incomplete-run',
			arguments: [
				'lint',
				'--config',
				casesConfigName,
				'--report',
				incompleteReport,
				'cases',
				casesPath,
				'--set',
				'development',
				'--case',
				'incomplete',
			],
			credential: true,
			expectedExit: 1,
		});
	} finally {
		options.observer.respond = previousResponder;
	}
	const incompleteCases = parseObject(filesystem.readFileSync(incompleteReport, 'utf8'));
	requireUnassessedBars(incompleteCases);
	requireEligibleFilter(incompleteCases, includedFolder);
	const incompleteRows = arrayValue(incompleteCases['cases']).map(objectValue);
	requireCaseAccounting({
		selectedIds: ['incomplete'],
		dispositions: incompleteRows.map((row) => ({
			id: textValue(row['id']),
			kind: textValue(objectValue(row['outcome'])['kind']),
		})),
		expectedKinds: new Map([['incomplete', 'incomplete']]),
	});
	assert.equal(
		incompleteRun.captures.length,
		2,
		'Expected one filter and one refused rule request',
	);
	const incompleteRaw = filesystem.readFileSync(incompleteReport, 'utf8');
	const captured = attributeControlledCaptures(
		jsonSlice(incompleteRaw, ['runs', 0, 'report']),
		incompleteRun.captures.map((entry) => entry.raw.toString('utf8')),
	);
	const incompleteFile = textValue(incompleteRule['file']);
	const expectedFilterOwner = JSON.stringify(['filter', 'category', includedFolder]);
	const expectedRuleOwner = JSON.stringify(['rule', 'asking_gate', incompleteFile, '1', '1']);
	assert.deepEqual(
		captured.map((entry) => entry.owner).toSorted(),
		[expectedFilterOwner, expectedRuleOwner].toSorted(),
		'Incomplete run request owners differ',
	);
	assert.ok(
		captured.find((entry) => entry.owner === expectedRuleOwner)?.raw.includes('GAP_BODY_SENTINEL'),
		'Refused rule request missed the authored sentinel',
	);
	const incompleteRuns = arrayValue(incompleteCases['runs']).map(objectValue);
	assert.equal(incompleteRuns.length, 1);
	const inner = objectValue(incompleteRuns[0]?.['report']);
	assert.equal(objectValue(inner['status'])['kind'], 'incomplete');
	const requests = arrayValue(inner['requests']).map(objectValue);
	const filter = requests.filter((request) => objectValue(request['owner'])['kind'] === 'filter');
	const rule = requests.filter((request) => objectValue(request['owner'])['kind'] === 'rule');
	assert.equal(filter.length, 1);
	assert.equal(rule.length, 1);
	assert.equal(objectValue(filter[0]?.['outcome'])['kind'], 'completed');
	const refusal = objectValue(rule[0]?.['outcome']);
	assert.equal(refusal['kind'], 'provider_failure');
	assert.equal(refusal['failure_class'], 'provider_rejected_request');
	assert.equal(refusal['http_status'], 400);
	const attempts = arrayValue(refusal['attempts']).map(objectValue);
	assert.equal(attempts.length, 1);
	assert.deepEqual(objectValue(attempts[0]?.['evidence']), {
		kind: 'remote_outcome_unknown',
		failure_class: 'provider_rejected_request',
		http_status: 400,
	});
	const results = arrayValue(inner['results']).map(objectValue);
	assert.equal(results.length, 1);
	assert.equal(results[0]?.['outcome'], 'incomplete');
	assert.equal(
		incompleteRows[0]?.['result_id'],
		results[0]?.['id'],
		'Incomplete case did not retain its unique result',
	);
	assert.notEqual(incompleteRows[0]?.['result_id'], null);
	assert.equal(results[0]?.['file'], incompleteFile);
}
