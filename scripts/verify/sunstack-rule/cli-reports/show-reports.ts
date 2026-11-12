/** Read real completed cases and ordinary/prepared neighbours through the signed native CLI. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireDebugSignature } from '../rulebook/rulebook-amendment.ts';
import {
	arrayValue,
	objectValue,
	numberValue,
	parseObject,
	readObject,
	runSequence,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import { requireAmbiguity, requireCaseDetail, requireLintDetail } from './report-checks.ts';
import {
	candidateForCase,
	candidatesFor,
	freezeCases,
	type SavedCandidate,
} from './report-inputs.ts';
import { checkedFile, configFile, prepareRepository } from './scope-fixtures.ts';

const schema = z.object({
	binary: z.string(),
	'source-root': z.string(),
	commit: z.string().regex(/^[a-f0-9]{40}$/),
	sha256: z.string().regex(/^[a-f0-9]{64}$/),
	evidence: z.string(),
	'expected-version': z
		.string()
		.regex(/^\d+\.\d+\.\d+$/)
		.default('0.11.1'),
	'cases-input': z.string(),
	'cases-sha256': z.string().regex(/^[a-f0-9]{64}$/),
	'unique-id': z.coerce.number().int().positive(),
	'ambiguous-id': z.coerce.number().int().positive(),
	'case-id': z.string().min(1),
});
function errorText(result: { stdout: Buffer; stderr: Buffer }): string {
	return Buffer.concat([result.stdout, result.stderr]).toString('utf8');
}
function requireNamedCasesError(text: string, selector: string): void {
	assert.match(text, /case/i);
	assert.ok(text.includes(selector), 'Refusal omitted the requested selector');
	assert.doesNotMatch(text, /missing field|invalid type:|unknown variant| at line \d+ column \d+/i);
}
export async function observeShow(options: z.infer<typeof schema>): Promise<void> {
	const target = declaredTarget({
		binary: options.binary,
		source: options['source-root'],
		commit: options.commit,
		digest: options.sha256,
	});
	requireDebugSignature(target.binary);
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const input = freezeCases({
		source: options['cases-input'],
		sha256: options['cases-sha256'],
		evidence,
	});
	const unique = candidatesFor(input.report, options['unique-id']);
	assert.equal(unique.length, 1, 'Declared unique result is not unique in the frozen input');
	const selectedUnique = unique[0];
	assert.ok(selectedUnique);
	const ambiguous = candidatesFor(input.report, options['ambiguous-id']);
	assert.ok(ambiguous.length > 1, 'Declared ambiguity control has no collision');
	const selectedCase = candidateForCase(input.report, options['case-id']);
	const expectedCase = arrayValue(input.report['cases'])
		.map(objectValue)
		.find((entry) => entry['id'] === options['case-id']);
	assert.ok(expectedCase);
	const empty = path.join(evidence, 'empty-cwd');
	filesystem.mkdirSync(empty);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		const show = async (
			name: string,
			file: string,
			selector: string,
			exit = 0,
			root = empty,
		): Promise<{ stdout: Buffer; stderr: Buffer }> => {
			const before = filesystem.readFileSync(file);
			const result = await observer.run({
				root,
				name,
				arguments: ['lint', 'show', file, selector],
				expectedExit: exit,
			});
			assert.equal(result.captures.length, 0);
			assert.deepEqual(filesystem.readFileSync(file), before, 'Show modified its saved input');
			assert.equal(
				createHash('sha256').update(filesystem.readFileSync(input.file)).digest('hex'),
				options['cases-sha256'],
				'Show changed the saved cases input',
			);
			return result;
		};
		const ordinaryRoot = path.join(evidence, 'ordinary-fixture');
		prepareRepository({ root: ordinaryRoot, endpoint: observer.endpoint });
		await observer.run({
			root: ordinaryRoot,
			name: 'ordinary-check',
			arguments: ['lint', '--config', configFile, '--check'],
			expectedExit: 0,
		});
		const ordinaryFile = path.join(evidence, 'ordinary.report.json');
		const produced = await observer.run({
			root: ordinaryRoot,
			name: 'ordinary-produce',
			arguments: [
				'lint',
				'--config',
				configFile,
				'--all',
				'--format',
				'json',
				'--report',
				ordinaryFile,
			],
			expectedExit: 0,
		});
		assert.equal(produced.captures.length, 0);
		const ordinary = readObject(ordinaryFile);
		assert.equal(ordinary['version'], options['expected-version']);
		const rows = arrayValue(ordinary['results']).map(objectValue);
		assert.equal(rows.length, 1);
		const row = rows[0];
		assert.ok(row);
		assert.equal(row['file'], checkedFile);
		assert.equal(row['outcome'], 'pass');
		assert.equal(objectValue(ordinary['usage'])['model_requests'], 0);
		const ordinaryCandidate: SavedCandidate = {
			revision: '',
			caseIds: [],
			report: ordinary,
			result: row,
		};
		const first = await show('ordinary-show', ordinaryFile, String(numberValue(row['id'])));
		requireLintDetail(parseObject(first.stdout.toString('utf8')), ordinaryCandidate);
		filesystem.writeFileSync(
			path.join(ordinaryRoot, checkedFile),
			'Changed checked content after report.\n',
		);
		filesystem.writeFileSync(path.join(ordinaryRoot, configFile), '{invalid current config');
		filesystem.writeFileSync(
			path.join(ordinaryRoot, 'rulebook.md'),
			'Changed rulebook after report.\n',
		);
		const after = await show(
			'ordinary-with-changed-current-files',
			ordinaryFile,
			String(numberValue(row['id'])),
			0,
			ordinaryRoot,
		);
		assert.deepEqual(
			after.stdout,
			first.stdout,
			'Ordinary show read current files/config instead of saved detail',
		);
		const nestedFile = path.join(evidence, 'r03-selected-run.lint.json');
		writeJson(nestedFile, selectedUnique.report);
		const numeric = await show('r03-unique-numeric', input.file, String(options['unique-id']));
		requireLintDetail(parseObject(numeric.stdout.toString('utf8')), selectedUnique);
		const direct = await show('nested-lint-unique', nestedFile, String(options['unique-id']));
		assert.deepEqual(
			numeric.stdout,
			direct.stdout,
			'Cases numeric detail differs from ordinary lint detail',
		);
		const named = await show('r03-case-id', input.file, options['case-id']);
		requireCaseDetail(parseObject(named.stdout.toString('utf8')), selectedCase, expectedCase);
		const caseRunFile = path.join(evidence, 'r03-case-run.lint.json');
		writeJson(caseRunFile, selectedCase.report);
		const caseDirect = await show(
			'nested-lint-case-result',
			caseRunFile,
			String(numberValue(selectedCase.result['id'])),
		);
		assert.deepEqual(
			objectValue(parseObject(named.stdout.toString('utf8'))['detail']),
			parseObject(caseDirect.stdout.toString('utf8')),
		);
		const collision = await show(
			'r03-ambiguous-numeric',
			input.file,
			String(options['ambiguous-id']),
			2,
		);
		requireAmbiguity(errorText(collision), options['ambiguous-id'], ambiguous);
		writeJson(
			path.join(evidence, 'ambiguity-oracle.json'),
			ambiguous.map((candidate) => ({
				revision: candidate.revision,
				case_ids: candidate.caseIds,
				result_id: candidate.result['id'] ?? null,
			})),
		);
		await runSequence(['999999', 'verifier-case-that-does-not-exist'], async (selector) => {
			requireNamedCasesError(
				errorText(await show(`unknown-${selector}`, input.file, selector, 2)),
				selector,
			);
		});
		const duplicateCaseFile = path.join(evidence, 'duplicate-case.json');
		const duplicateCase = structuredClone(input.report);
		arrayValue(duplicateCase['cases']).push({ ...expectedCase, result_id: options['unique-id'] });
		writeJson(duplicateCaseFile, duplicateCase);
		requireNamedCasesError(
			errorText(await show('duplicate-case-id', duplicateCaseFile, options['case-id'], 2)),
			options['case-id'],
		);
		const duplicateRunFile = path.join(evidence, 'duplicate-run.json');
		const duplicateRun = structuredClone(input.report);
		const run = arrayValue(duplicateRun['runs'])
			.map(objectValue)
			.find((entry) => entry['revision'] === selectedUnique.revision);
		assert.ok(run);
		arrayValue(duplicateRun['runs']).push(structuredClone(run));
		writeJson(duplicateRunFile, duplicateRun);
		requireNamedCasesError(
			errorText(
				await show('duplicate-revision', duplicateRunFile, String(options['unique-id']), 2),
			),
			String(options['unique-id']),
		);
		const unknownKind = path.join(evidence, 'unknown-kind.json');
		writeJson(unknownKind, { unrelated: true });
		assert.match(errorText(await show('unknown-report-kind', unknownKind, '1', 2)), /report/i);
		await observePrepared({ observer, evidence, endpoint: observer.endpoint, show });
		assert.equal(observer.captures.length, 0);
		writeJson(path.join(evidence, 'pass.json'), {
			target,
			cases_input_sha256: options['cases-sha256'],
			unique_id: options['unique-id'],
			case_id: options['case-id'],
			ambiguous_id: options['ambiguous-id'],
			candidates: ambiguous.map((candidate) => ({
				revision: candidate.revision,
				case_ids: candidate.caseIds,
			})),
			ordinary_saved_detail_preserved: true,
			prepared_ordinals_preserved: true,
			http_requests: 0,
			manual_check:
				'Compare candidate associations in r03-ambiguous-numeric/stderr.log against ambiguity-oracle.json; automatic checks verify all candidate identities are present.',
			limits: [
				'Saved readback only; no new inference or classification-quality evidence.',
				'Original report endpoints are not rewritten; loopback is not an egress witness for pure saved readback.',
			],
		});
	} finally {
		await observer.close();
	}
}
async function observePrepared(options: {
	observer: RulebookProcess;
	evidence: string;
	endpoint: string;
	show: (
		name: string,
		file: string,
		selector: string,
		exit?: number,
		root?: string,
	) => Promise<{ stdout: Buffer; stderr: Buffer }>;
}): Promise<void> {
	const root = path.join(options.evidence, 'prepared-fixture');
	const revision = prepareRepository({ root, endpoint: options.endpoint });
	const definitions = ['docs/peer.md', checkedFile].map((file, index) => ({
		id: index === 0 ? 'prepared-no-linked-result' : 'prepared-linked-result',
		revision,
		set: 'development',
		label_from: 'owner-ruling',
		kind: 'rule',
		rule: 'body-present',
		file,
		lines: [1, 1],
		text_sha256: createHash('sha256')
			.update(filesystem.readFileSync(path.join(root, file)))
			.digest('hex'),
		expected: 'pass',
	}));
	const caseFile = path.join(options.evidence, 'prepared-fixture.cases.json');
	writeJson(caseFile, { repository: root, cases: definitions, bars: [] });
	writeJson(path.join(options.evidence, 'prepared-fixture.provenance.json'), {
		synthetic: true,
		purpose: 'Prepared ordinal2 must resolve nested result1 while ordinal1 has no linked result.',
		label_from:
			'Required fixture wire metadata only; this does not establish an owner ruling or classifier quality.',
	});
	const preparedFile = path.join(options.evidence, 'prepared-cases.json');
	await options.observer.run({
		root,
		name: 'prepared-check',
		arguments: ['lint', '--config', configFile, '--check'],
		expectedExit: 0,
	});
	await options.observer.run({
		root,
		name: 'prepared-cases-produce',
		arguments: ['lint', '--config', configFile, '--prepare', preparedFile, 'cases', caseFile],
		expectedExit: 1,
	});
	const prepared = readObject(preparedFile);
	assert.equal(prepared['mode'], 'prepare');
	const cases = arrayValue(prepared['cases']).map(objectValue);
	assert.equal(cases.length, 2);
	assert.equal(cases[0]?.['result_id'], null);
	assert.equal(cases[1]?.['result_id'], 1, 'Ordinal/result namespace control is inert');
	const first = parseObject(
		(await options.show('prepared-ordinal-1', preparedFile, '1')).stdout.toString('utf8'),
	);
	assert.deepEqual(first, { case: cases[0], revision });
	const second = parseObject(
		(await options.show('prepared-ordinal-2', preparedFile, '2')).stdout.toString('utf8'),
	);
	assert.deepEqual(second['case'], cases[1]);
	assert.equal(second['revision'], revision);
	assert.equal(objectValue(objectValue(second['detail'])['result'])['id'], 1);
	requireNamedCasesError(
		errorText(await options.show('prepared-ordinal-unknown', preparedFile, '3', 2)),
		'3',
	);
}
if (process.argv[1] === import.meta.filename) {
	const { values } = parseArgs({
		options: {
			help: { type: 'boolean' },
			...Object.fromEntries(
				[
					'binary',
					'source-root',
					'commit',
					'sha256',
					'evidence',
					'cases-input',
					'cases-sha256',
					'unique-id',
					'ambiguous-id',
					'case-id',
					'expected-version',
				].map((name) => [name, { type: 'string' as const }]),
			),
		},
	});
	if (values.help)
		process.stdout.write(
			'Read actual cases-report selectors and ordinary/prepared saved detail without config/credentials.\n--binary PATH --source-root CHECKOUT --commit FULL_SHA --sha256 DIGEST --evidence NEW_DIRECTORY [--expected-version VERSION] --cases-input REPORT --cases-sha256 DIGEST --unique-id N --ambiguous-id N --case-id NAME\nR03 selectors: unique41, ambiguous27, case r03-current-typescript-topics. Verifies payload identity, collision candidates, current-file independence and prepared ordinals. Owns bounded children/listener; no inference. Candidate association in the named ambiguity output must also be read against ambiguity-oracle.json.\n',
		);
	else await observeShow(schema.parse(values));
}
