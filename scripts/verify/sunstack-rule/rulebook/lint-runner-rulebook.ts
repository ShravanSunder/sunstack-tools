/** Retained lint exits, formats, no-key admission, init and explicit/default scopes. */
import filesystem from 'node:fs';
import path from 'node:path';

import { z } from 'zod';

import {
	arrayValue,
	configName,
	configure,
	configureEndpoint,
	equal,
	errorMessage,
	fixtureSource,
	journeyArguments,
	runSequence,
	lintArguments,
	objectValue,
	oracle,
	parseObject,
	prepareFixture,
	readObject,
	requireCondition,
	rulesName,
	selectWorkedSkill,
	skillName,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import { findingLine, reportFindings, reportStatus } from './rulebook-lint.ts';
import { declaredTarget, responseBody, RulebookProcess } from './rulebook-process.ts';

export const lintCaseSchema = z.object({
	name: z.string(),
	exit: z.number().int(),
	status: z.string().optional(),
	arguments: z.array(z.string()).default(() => lintArguments('--all')),
	severity: z.string().optional(),
	marker: z.string().default('K002'),
	requests: z.number().int().default(0),
	credential: z.boolean().default(false),
	cancel: z.boolean().default(false),
	format: z.string().default('json'),
});
export type LintCase = z.infer<typeof lintCaseSchema>;
export function prepareLint(destination: string): LintCase[] {
	filesystem.mkdirSync(destination);
	const cases = [
		{ name: 'mechanical-error', exit: 1, status: 'findings', severity: 'error' },
		{ name: 'clean', exit: 0, status: 'clean' },
		{ name: 'warning', exit: 0, status: 'clean', severity: 'warn' },
		{
			name: 'warning-limit',
			exit: 1,
			status: 'findings',
			severity: 'warn',
			arguments: lintArguments('--all', '--max-warnings', '0'),
		},
		{ name: 'ignored', exit: 0, status: 'nothing_selected' },
		{ name: 'empty-candidates', exit: 0, status: 'nothing_selected' },
		{ name: 'missing-key', exit: 2, status: 'invalid', marker: 'K003' },
		{
			name: 'judged',
			exit: 1,
			status: 'findings',
			severity: 'error',
			marker: 'K003',
			requests: 1,
			credential: true,
		},
		{
			name: 'cancelled',
			exit: 4,
			status: 'cancelled',
			marker: 'K003',
			requests: 1,
			credential: true,
			cancel: true,
		},
		{ name: 'file-limit', exit: 3, status: 'incomplete' },
		{ name: 'check', exit: 0, arguments: ['lint', '--config', configName, '--check'] },
		{ name: 'invalid', exit: 2, status: 'invalid' },
		{
			name: 'named-path',
			exit: 1,
			status: 'findings',
			severity: 'error',
			arguments: lintArguments(skillName),
		},
		{
			name: 'default-discovery',
			exit: 1,
			status: 'findings',
			severity: 'error',
			arguments: ['lint', '--all', '--format', 'json'],
		},
		{ name: 'init', exit: 0, arguments: ['lint', '--init'] },
		...['default', 'github', 'sarif'].map((format) => ({
			name: `format-${format}`,
			exit: 1,
			severity: 'error',
			format,
			arguments: ['lint', '--config', configName, '--all', '--format', format],
		})),
	].map((value) => lintCaseSchema.parse(value));
	for (const scenario of cases) {
		const root = path.join(destination, scenario.name);
		if (scenario.name === 'init') {
			filesystem.mkdirSync(root);
			continue;
		}
		prepareFixture(root, [scenario.marker]);
		selectWorkedSkill(root);
		const rules = readObject(path.join(root, rulesName));
		const entry = objectValue(rules[scenario.marker]);
		if (['warning', 'warning-limit'].includes(scenario.name)) entry['severity'] = 'warn';
		if (scenario.marker === 'K003') entry['severity'] = 'error';
		writeJson(path.join(root, rulesName), rules);
		if (scenario.name === 'clean') {
			const file = path.join(root, skillName);
			filesystem.writeFileSync(
				file,
				filesystem
					.readFileSync(file, 'utf8')
					.replace(/^description:.*$/m, 'description: "Use for a short description."'),
			);
		}
		if (scenario.name === 'ignored')
			configure(root, (config) => {
				objectValue(config['check'])['ignore'] = [skillName];
			});
		if (scenario.name === 'empty-candidates')
			configure(root, (config) => {
				objectValue(config['check'])['files'] = ['absent/*.md'];
			});
		if (scenario.name === 'file-limit')
			configure(root, (config) => {
				config['limits'] = { max_file_bytes: 1 };
			});
		if (scenario.name === 'invalid')
			configure(root, (config) => {
				config['unknown'] = true;
			});
		if (scenario.name === 'default-discovery')
			filesystem.copyFileSync(path.join(root, configName), path.join(root, 'sunstack.lint.json'));
		writeJson(path.join(root, 'expected-results.json'), scenario);
	}
	writeJson(path.join(destination, 'manifest.json'), cases);
	return cases;
}
export function requireReport(report: JsonObject, scenario: LintCase): void {
	equal(reportStatus(report), scenario.status, `${scenario.name}: wrong status`);
	const rows = reportFindings(report);
	const counts = objectValue(report['counts']);
	for (const severity of ['error', 'warn'])
		equal(
			counts[severity],
			rows.filter((item) => item['severity'] === severity).length,
			'Severity counts differ',
		);
	if (scenario.severity) {
		const item = rows[0];
		requireCondition(item && rows.length === 1, 'Extra or missing findings');
		equal(
			[item['marker'], item['file'], findingLine(item), item['severity']],
			[scenario.marker, skillName, 3, scenario.severity],
			'Wrong finding identity/location/severity',
		);
		requireCondition(item['message'], 'Finding has no user-visible message');
	} else equal(rows, [], 'Unexpected findings');
	const usage = objectValue(report['usage']);
	equal(usage['model_requests'], scenario.requests, 'Reported requests differ');
	if (['missing-key', 'invalid'].includes(scenario.name))
		equal(usage['checked_files_acquired'], 0, 'Invalid/no-key run acquired checked content');
	if (scenario.name === 'judged') {
		requireCondition(
			Object.keys(objectValue(report['evidence_seen'])).length > 0,
			'Judged evidence missing',
		);
		const item = rows[0];
		requireCondition(item, 'Missing judged finding');
		const answer = objectValue(
			objectValue(objectValue(item['judgment'])['answers'])['names_trigger'],
		);
		equal(
			[answer['type'], answer['yes_probability']],
			['noul', 0.1],
			'Native supplied answer lost',
		);
	}
	if (['cancelled', 'file-limit'].includes(scenario.name))
		requireCondition(arrayValue(report['incomplete']).length > 0, 'Unresolved work missing');
}
async function observeInit(observer: RulebookProcess, root: string): Promise<void> {
	await observer.run({ root, name: 'init-create', arguments: ['lint', '--init'], expectedExit: 0 });
	const file = path.join(root, 'sunstack.lint.json');
	const skeleton = readObject(file);
	requireCondition(
		'rulebook' in skeleton && !('rule_sets' in skeleton),
		'Init produced old config',
	);
	equal(filesystem.readdirSync(root), ['sunstack.lint.json'], 'Init wrote unexpected files');
	const original = filesystem.readFileSync(file);
	await observer.run({
		root,
		name: 'init-no-overwrite',
		arguments: ['lint', '--init'],
		expectedExit: 2,
	});
	equal(filesystem.readFileSync(file), original, 'Init overwrote bytes');
	await observer.run({
		root,
		name: 'init-empty-check',
		arguments: ['lint', '--check'],
		expectedExit: 2,
	});
	filesystem.cpSync(fixtureSource, root, { recursive: true });
	filesystem.copyFileSync(path.join(root, configName), file);
	await observer.run({
		root,
		name: 'init-populated-check',
		arguments: ['lint', '--check'],
		expectedExit: 0,
	});
}
export async function main(): Promise<void> {
	const options = journeyArguments('S1 current lint exits, formats, admission and init.');
	if (!options) return;
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
	const cases = prepareLint(path.join(evidence, 'fixtures'));
	const selected = cases.filter(
		(scenario) => !options.case.length || options.case.includes(scenario.name),
	);
	requireCondition(
		selected.length > 0 &&
			options.case.every((name) => cases.some((scenario) => scenario.name === name)),
		'Unknown/empty case selection',
	);
	if (options.prepare) {
		writeJson(path.join(evidence, 'prepared.json'), {
			product_observed: false,
			cases: cases.length,
			oracle,
		});
		process.stdout.write(`Prepared ${cases.length} S1 cases; product unobserved\n`);
		return;
	}
	requireCondition(target, 'Declared target missing');
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	observer.reply = (body) => responseBody(body, 0.1);
	try {
		await runSequence(selected, async (scenario) => {
			const root = path.join(evidence, 'fixtures', scenario.name);
			if (scenario.name === 'init') {
				await observeInit(observer, root);
				return;
			}
			configureEndpoint(root, observer.endpoint);
			if (scenario.name === 'default-discovery')
				filesystem.copyFileSync(path.join(root, configName), path.join(root, 'sunstack.lint.json'));
			const result = await observer.run({
				root,
				name: scenario.name,
				arguments: scenario.arguments,
				expectedExit: scenario.exit,
				credential: scenario.credential,
				cancel: scenario.cancel,
			});
			equal(result.captures.length, scenario.requests, 'Actual HTTP calls differ');
			if (scenario.status) requireReport(parseObject(result.stdout.toString('utf8')), scenario);
			else if (scenario.name === 'check')
				requireCondition(
					Buffer.concat([result.stdout, result.stderr]).includes('K002'),
					'Check lost listing',
				);
			else if (scenario.format === 'sarif') {
				const runs = arrayValue(parseObject(result.stdout.toString('utf8'))['runs']);
				equal(
					arrayValue(objectValue(runs[0])['results']).length,
					1,
					'SARIF finding lost/duplicated',
				);
			} else
				requireCondition(
					result.stdout.includes('Over 300 characters.') &&
						(result.stdout.includes(skillName) || result.stdout.includes('SKILL.md')),
					'Rendered finding/message lost',
				);
		});
	} finally {
		await observer.close();
	}
	writeJson(path.join(evidence, 's1-pass.json'), {
		commit: target.commit,
		binary_sha256: target.digest,
		executed: selected.map((scenario) => scenario.name),
		full_inventory: cases.length,
		required_joins: [
			'rulebook-lint Git/change/skip',
			'native report schema and library/CLI equality',
			'native exec JSONL/conversation tests',
			'recording-source gate tests',
			'Linux/container and released schema separate targets',
		],
	});
	process.stdout.write(`Observed ${selected.length} S1 cases; native/target joins named\n`);
}
if (import.meta.main)
	try {
		await main();
	} catch (error) {
		process.stderr.write(`S1 rulebook observation failed: ${errorMessage(error)}\n`);
		process.exitCode = 2;
	}
