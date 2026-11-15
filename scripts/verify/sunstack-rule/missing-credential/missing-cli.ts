/** O1/O2/O4 through actual CLI processes, including their own raw saved format. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import { baselineGit, runSequence } from '../rulebook/rulebook-fixture-support.ts';
import { type ReleasedTarget, type RulebookProcess } from '../rulebook/rulebook-process.ts';
import {
	integer,
	record,
	rows,
	requireCaseLinks,
	requireLocalParity,
	requireProviderFailure,
	requireRequestParity,
	requireUnavailableResults,
	requireUnavailableSelection,
	requireZeroDispatch,
	status,
	text,
} from './missing-checks.ts';
import {
	judgedNames,
	localNames,
	sourceDigest,
	sourceFile,
	writeCases,
	writeFixture,
} from './missing-fixtures.ts';

export interface CliQualification {
	readonly reportFile: string;
	readonly report: Record<string, unknown>;
	readonly caseFile: string;
	readonly caseReport: Record<string, unknown>;
	readonly root: string;
}
export async function observeMissingCli(
	observer: RulebookProcess,
	evidence: string,
	reference: ReleasedTarget,
): Promise<CliQualification> {
	const root = path.join(evidence, 'cli-fixture');
	const run = async (
		name: string,
		expectedExit: number,
		credential = false,
		extra: string[] = [],
	): Promise<{
		report: Record<string, unknown>;
		raw: string;
		captures: string[];
		file: string;
	}> => {
		const file = path.join(evidence, `${name}-report.json`);
		const result = await observer.run({
			root,
			name,
			arguments: [
				'lint',
				'--config',
				'sunstack.lint.json',
				'--format',
				'json',
				'--all',
				'--report',
				file,
				...extra,
			],
			expectedExit,
			credential,
		});
		const raw = filesystem.readFileSync(file, 'utf8');
		return {
			report: record(JSON.parse(raw)),
			raw,
			captures: result.captures.map((capture) => capture.raw.toString('utf8')),
			file,
		};
	};
	writeFixture(root, observer.endpoint, 'mixed');
	const casesInput = path.join(evidence, 'cases-input.json');
	writeCases(root, casesInput);
	const keyed = await run('keyed-witness', 1, true);
	assert.ok(keyed.captures.length > 0, 'No live keyed CLI witness');
	assert.ok(integer(record(keyed.report['usage'])['model_requests']) > 0n);
	const baseline = await observer.run({
		target: reference,
		root,
		name: 'keyed-released-reference',
		arguments: ['lint', '--config', 'sunstack.lint.json', '--all', '--format', 'json'],
		expectedExit: 1,
		credential: true,
	});
	requireRequestParity(
		baseline.captures.map((capture) => capture.raw.toString('utf8')),
		keyed.captures,
	);
	writeFixture(root, observer.endpoint, 'mechanical');
	const mechanical = await run('mechanical-neighbor', 1);
	writeFixture(root, observer.endpoint, 'mixed');
	const mixed = await run('mixed-keyless', 3);
	assert.equal(status(mixed.report), 'incomplete');
	requireLocalParity({
		mechanical: mechanical.report,
		mixed: mixed.report,
		names: localNames,
		expectedFindings: 3,
		expectedSha256: sourceDigest,
	});
	requireUnavailableResults(
		mixed.report,
		new Map(judgedNames.map((name) => [name, 'credential_unavailable'])),
	);
	requireZeroDispatch(mixed.report, mixed.captures);
	const human = await observer.run({
		root,
		name: 'mixed-human',
		arguments: ['lint', '--config', 'sunstack.lint.json', '--all'],
		expectedExit: 3,
	});
	for (const name of localNames)
		assert.ok(human.stdout.toString('utf8').includes(`Original ${name} finding.`));
	assert.match(human.stdout.toString('utf8'), /unavailable/iu);

	await runSequence(['filter', 'selection'] as const, async (mode) => {
		writeFixture(root, observer.endpoint, mode);
		const result = await run(`${mode}-keyless`, 3);
		requireZeroDispatch(result.report, result.captures);
		const expected = new Map<string, string>(
			judgedNames.map((name) => [name, 'credential_unavailable']),
		);
		expected.set(
			mode === 'filter' ? 'gated_local' : 'needed_field',
			mode === 'filter' ? 'filter_unavailable' : 'selected_field_unavailable',
		);
		requireUnavailableResults(result.report, expected);
		if (mode === 'filter') {
			const answers = rows(result.report['filter_answers']);
			assert.equal(answers.length, 1);
			assert.equal(answers[0]?.['state'], 'unavailable');
		} else {
			requireUnavailableSelection(result.report);
			assert.ok(!rows(result.report['results']).some((row) => row['name'] === 'selected_child'));
			assert.equal(
				rows(result.report['results']).find((row) => row['name'] === 'unused_field')?.['outcome'],
				'finding',
			);
			assert.ok(
				rows(result.report['incomplete']).some(
					(row) => record(row['reason'])['kind'] === 'selection_unavailable',
				),
			);
			for (const selection of rows(result.report['selections']))
				assert.equal(record(selection['outcome'])['kind'], 'unavailable');
			const unknownCases = path.join(evidence, 'unknown-cases-input.json');
			writeCases(root, unknownCases, 'selected_child');
			const unknownReportFile = path.join(evidence, 'unknown-cases-report.json');
			const actual = await observer.run({
				root,
				name: 'unknown-child-cases',
				arguments: [
					'lint',
					'--config',
					'sunstack.lint.json',
					'--report',
					unknownReportFile,
					'cases',
					unknownCases,
					'--set',
					'development',
				],
				expectedExit: 1,
			});
			assert.equal(actual.captures.length, 0);
			const unknownReport = record(JSON.parse(filesystem.readFileSync(unknownReportFile, 'utf8')));
			requireCaseLinks({
				cases: unknownReport['cases'],
				expected: new Map(
					['known-a', 'known-b'].map((id) => [id, { kind: 'unavailable', linked: false }]),
				),
				actualResultIds: [],
			});
		}
	});
	writeFixture(root, observer.endpoint, 'mixed');
	const casesOutput = path.join(evidence, 'cases-report.json');
	const cases = await observer.run({
		root,
		name: 'cases-keyless',
		arguments: [
			'lint',
			'--config',
			'sunstack.lint.json',
			'--report',
			casesOutput,
			'cases',
			casesInput,
			'--set',
			'development',
			'--case',
			'known-a',
			'--case',
			'known-b',
		],
		expectedExit: 1,
	});
	assert.equal(cases.captures.length, 0);
	const caseReport = record(JSON.parse(filesystem.readFileSync(casesOutput, 'utf8')));
	const nested = rows(caseReport['runs']).map((row) => record(row['report']));
	requireCaseLinks({
		cases: caseReport['cases'],
		expected: new Map(
			['known-a', 'known-b'].map((id) => [id, { kind: 'unavailable', linked: true }]),
		),
		actualResultIds: nested.flatMap((report) => rows(report['results']).map((row) => row['id'])),
	});

	await runSequence([401, 400, 503], async (code) => {
		observer.respond = async () => ({
			status: code,
			body: { error: { message: `Controlled ${code}` } },
		});
		const result = await run(`http-${code}`, 3, true);
		requireProviderFailure(result.report, code, result.captures.length);
		assert.ok(result.captures.length > 0, 'Failure path never submitted HTTP');
		assert.equal(
			rows(result.report['results']).filter((row) => row['outcome'] === 'unavailable').length,
			0,
			'Real HTTP failure became credential unavailable',
		);
		assert.ok(rows(result.report['requests']).length > 0, 'Actual failure lost request evidence');
		assert.ok(integer(record(result.report['usage'])['model_requests']) > 0n);
	});
	observer.respond = undefined;
	await observer.run({
		root,
		name: 'cancel-held-cli',
		arguments: ['lint', '--config', 'sunstack.lint.json', '--all'],
		expectedExit: 4,
		credential: true,
		cancel: true,
	});
	await observer.run({
		root,
		name: 'static-check',
		arguments: ['lint', '--config', 'sunstack.lint.json', '--check'],
		expectedExit: 0,
	});
	const prepared = path.join(evidence, 'prepared.json');
	const prepare = await observer.run({
		root,
		name: 'explicit-prepare',
		arguments: ['lint', '--config', 'sunstack.lint.json', '--all', '--prepare', prepared],
		expectedExit: 6,
	});
	assert.equal(prepare.captures.length, 0);
	assert.equal(record(JSON.parse(filesystem.readFileSync(prepared, 'utf8')))['mode'], 'prepare');
	const listing = await observer.run({
		root,
		name: 'static-inspection',
		arguments: ['lint', '--config', 'sunstack.lint.json', '--all', '--evidence'],
		expectedExit: 0,
	});
	assert.equal(listing.captures.length, 0);
	assert.ok(rows(record(JSON.parse(listing.stdout.toString('utf8')))['pieces']).length > 0);
	writeFixture(root, observer.endpoint, 'selection');
	const denied = await observer.run({
		root,
		name: 'selected-inspection-keyless',
		arguments: ['lint', '--config', 'sunstack.lint.json', '--all', '--evidence'],
		expectedExit: 2,
	});
	assert.equal(denied.captures.length, 0);
	const selected = await observer.run({
		root,
		name: 'selected-inspection-keyed',
		arguments: ['lint', '--config', 'sunstack.lint.json', '--all', '--evidence'],
		expectedExit: 0,
		credential: true,
	});
	assert.ok(selected.captures.length > 0);
	writeFixture(root, observer.endpoint, 'invalid');
	await run('invalid-global', 2);
	writeFixture(root, observer.endpoint, 'mixed');
	// Commit current valid fixture once so since HEAD has a genuinely empty changed scope.
	baselineGit(root);
	const empty = await observer.run({
		root,
		name: 'empty-since',
		arguments: ['lint', '--config', 'sunstack.lint.json', '--since', 'HEAD', '--format', 'json'],
		expectedExit: 0,
	});
	assert.equal(empty.captures.length, 0);
	assert.equal(status(record(JSON.parse(empty.stdout.toString('utf8')))), 'nothing_selected');
	assert.equal(filesystem.readFileSync(path.join(root, sourceFile), 'utf8').length > 0, true);
	return { reportFile: mixed.file, report: mixed.report, caseFile: casesOutput, caseReport, root };
}

export async function showDeletedCli(
	observer: RulebookProcess,
	saved: CliQualification,
): Promise<void> {
	await runSequence(rows(saved.report['results']), async (result) => {
		const detail = await observer.run({
			root: saved.root,
			name: `show-result-${text(String(result['id']))}`,
			arguments: ['lint', 'show', saved.reportFile, String(result['id'])],
			expectedExit: 0,
		});
		assert.equal(detail.captures.length, 0);
		const shown = record(JSON.parse(detail.stdout.toString('utf8')));
		assert.deepEqual(shown['result'], result, 'Raw saved result changed after source deletion');
	});
	await runSequence(['known-a', 'known-b'], async (id) => {
		const detail = await observer.run({
			root: saved.root,
			name: `show-case-${id}`,
			arguments: ['lint', 'show', saved.caseFile, id],
			expectedExit: 0,
		});
		assert.equal(detail.captures.length, 0);
		assert.match(detail.stdout.toString('utf8'), /unavailable/u);
	});
}
