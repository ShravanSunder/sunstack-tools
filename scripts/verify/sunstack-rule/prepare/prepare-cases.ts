/** Revision-bound prepare accounting with historical-source and obsolete-definition controls. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import { prepareCasesFixture, casesConfigName } from '../rulebook/rulebook-case-fixtures.ts';
import {
	arrayValue,
	gitCommand,
	objectValue,
	parseObject,
	readObject,
	runSequence,
	textValue,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import { type RulebookProcess } from '../rulebook/rulebook-process.ts';

export async function observePrepareCases(options: {
	observer: RulebookProcess;
	evidence: string;
}): Promise<void> {
	const root = path.join(options.evidence, 'cases-fixture');
	const fixture = prepareCasesFixture(root);
	const current = textValue(fixture['current']);
	const previous = textValue(fixture['historical']);
	gitCommand(root, ['switch', '--detach', previous]);
	const obsolete = 'evidence/rules/obsolete-prepare-history.yml';
	filesystem.writeFileSync(path.join(root, obsolete), 'not a rule mapping\n');
	gitCommand(root, ['add', obsolete]);
	gitCommand(root, ['commit', '-qm', 'Historical obsolete evidence rule']);
	const historical = gitCommand(root, ['rev-parse', 'HEAD']);
	gitCommand(root, ['switch', '--detach', current]);
	const definitions = [...arrayValue(fixture['rule_cases']), ...arrayValue(fixture['filter_cases'])]
		.map(objectValue)
		.map((entry) => {
			if (entry['revision'] === previous) entry['revision'] = historical;
			return entry;
		});
	const configPath = path.join(root, casesConfigName);
	const config = readObject(configPath);
	objectValue(config['decision_model'])['endpoint'] = options.observer.endpoint;
	writeJson(configPath, config);
	const input = path.join(options.evidence, 'prepare-cases-input.json');
	writeJson(input, { repository: root, cases: definitions, bars: fixture['bars'] });
	const output = path.join(options.evidence, 'cases.prepared.json');
	const result = await options.observer.run({
		root,
		name: 'prepare-cases',
		arguments: [
			'lint',
			'--config',
			casesConfigName,
			'cases',
			input,
			'--set',
			'development',
			'--prepare',
			output,
		],
		expectedExit: 1,
	});
	assert.equal(result.captures.length, 0, 'Preparing revision cases dispatched HTTP');
	const report = parseObject(filesystem.readFileSync(output, 'utf8'));
	assert.equal(report['mode'], 'prepare');
	assert.deepEqual(report['errors'], {});
	const rows = arrayValue(report['cases']).map(objectValue);
	assert.equal(rows.length, 16);
	assert.deepEqual(
		rows.map((row) => textValue(row['id'])).toSorted(),
		definitions.map((entry) => textValue(entry['id'])).toSorted(),
		'A selected case disappeared or repeated',
	);
	for (const row of rows) {
		const definition = definitions.find((entry) => entry['id'] === row['id']);
		assert.ok(definition);
		assert.deepEqual(row['case'], definition, 'Case hash/range/label changed during prepare');
		const expected =
			row['id'] === 'stale'
				? 'stale'
				: ['unmatched', 'ambiguous'].includes(textValue(row['id']))
					? 'no_matching_piece'
					: 'prepared';
		assert.equal(
			objectValue(row['outcome'])['kind'],
			expected,
			`Wrong prepare disposition for ${textValue(row['id'])}`,
		);
		if (expected !== 'prepared')
			assert.equal(row['result_id'], null, 'Unmatched or stale case fabricated a result');
	}
	assert.deepEqual(report['counts'], { prepared: 13, stale: 1, no_matching_piece: 2 });
	const runs = arrayValue(report['runs']).map(objectValue);
	assert.equal(runs.length, 2);
	for (const run of runs) {
		const lint = objectValue(run['report']);
		assert.equal(lint['mode'], 'prepare');
		assert.equal(objectValue(lint['usage'])['model_requests'], 0);
		const raw = JSON.stringify(lint);
		assert.ok(
			!raw.includes('obsolete-prepare-history.yml'),
			'Historical obsolete definition survived overlay',
		);
		const historic = run['revision'] === historical;
		const requests = JSON.stringify(lint['requests']);
		for (const sentinel of historic
			? ['HISTORICAL_REFERENCE_BODY_SENTINEL', 'HISTORICAL_EXTERNAL_OPENING_SENTINEL']
			: ['CURRENT_REFERENCE_BODY_SENTINEL', 'CURRENT_EXTERNAL_OPENING_SENTINEL'])
			assert.ok(requests.includes(sentinel), 'Prepared case request read wrong revision context');
		assert.ok(
			requests.includes('CURRENT_RULEBOOK_REFERENCE_SENTINEL'),
			'Current rulebook did not overlay historical tree',
		);
	}
	const showRows = runs.map((run) => {
		const requests = arrayValue(objectValue(run['report'])['requests']).map(objectValue);
		const index = rows.findIndex(
			(row) =>
				objectValue(row['case'])['revision'] === run['revision'] &&
				objectValue(row['outcome'])['kind'] === 'prepared' &&
				requests.some((request) => request['id'] === row['result_id']),
		);
		assert.ok(index >= 0, 'Revision lacks a prepared request readback witness');
		return index;
	});
	const staleIndex = rows.findIndex((row) => row['id'] === 'stale');
	assert.ok(staleIndex >= 0);
	showRows.push(staleIndex);
	const before = new Map<number, Buffer>();
	await runSequence(showRows, async (index) => {
		const row = rows[index];
		assert.ok(row);
		const shown = await options.observer.run({
			root,
			name: `show-case-${index + 1}`,
			arguments: ['lint', 'show', output, String(index + 1)],
			expectedExit: 0,
		});
		assert.equal(shown.captures.length, 0);
		before.set(index, shown.stdout);
		const detail = parseObject(shown.stdout.toString('utf8'));
		assert.deepEqual(detail['case'], row);
		assert.equal(detail['revision'], objectValue(row['case'])['revision']);
		if (index === staleIndex) assert.ok(!('detail' in detail));
		else {
			const revisionRun = runs.find((candidate) => candidate['revision'] === detail['revision']);
			assert.ok(revisionRun);
			const lint = objectValue(revisionRun['report']);
			const linkedRequest = arrayValue(lint['requests'])
				.map(objectValue)
				.find((candidate) => candidate['id'] === row['result_id']);
			assert.ok(linkedRequest);
			assert.deepEqual(objectValue(detail['detail'])['request'], linkedRequest);
			assert.deepEqual(objectValue(detail['detail'])['rulebook_files'], lint['rulebook_files']);
		}
	});
	for (const file of [casesConfigName, 'rules.json', 'rulebook.md'])
		filesystem.unlinkSync(path.join(root, file));
	await runSequence(showRows, async (index) => {
		const shown = await options.observer.run({
			root,
			name: `show-case-without-source-${index + 1}`,
			arguments: ['lint', 'show', output, String(index + 1)],
			expectedExit: 0,
		});
		assert.equal(shown.captures.length, 0);
		assert.deepEqual(shown.stdout, before.get(index), 'Case readback depended on current sources');
	});
	await runSequence([0, rows.length + 1], async (ordinal) => {
		const refused = await options.observer.run({
			root,
			name: `show-case-invalid-${ordinal}`,
			arguments: ['lint', 'show', output, String(ordinal)],
			expectedExit: 2,
		});
		assert.equal(refused.captures.length, 0);
		assert.match(refused.stderr.toString('utf8'), /unknown case ordinal/);
	});
	writeJson(path.join(options.evidence, 'cases-observed.json'), {
		historical,
		current,
		selected_cases: rows.length,
		counts: report['counts'],
		zero_http: true,
		case_readback_ordinals: showRows.map((index) => index + 1),
		invalid_ordinal_exit: 2,
		self_contained_readback: true,
		source_rule_overlay: 'native engine, no manual purge',
	});
}
