/** 255-option locator boundary and reached-result omission through a declared signed CLI. */
import filesystem from 'node:fs';
import path from 'node:path';

import { requireDebugSignature } from './rulebook-amendment.ts';
import {
	arrayValue,
	configName,
	configure,
	configureEndpoint,
	equal,
	errorMessage,
	journeyArguments,
	lintArguments,
	objectValue,
	requireCondition,
	rulebookName,
	rulesName,
	runSequence,
	textValue,
	writeJson,
	type JourneyArguments,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import {
	declaredTarget,
	responseBody,
	RulebookProcess,
	type CapturedRequest,
} from './rulebook-process.ts';

export type LocatorScenario =
	| 'lines254'
	| 'lines255'
	| 'over-limit'
	| 'low'
	| 'no-locator'
	| 'pass'
	| 'mechanical';
export const locatorScenarios: readonly LocatorScenario[] = [
	'lines254',
	'lines255',
	'over-limit',
	'low',
	'no-locator',
	'pass',
	'mechanical',
];
const subjectFile = 'documents/subject.md';
const peerFile = 'documents/peer.md';
const firstCandidateLine = 3;
const locatorName = 'focus_line';
const authoredQuestion: JsonObject = {
	type: 'noul',
	instructions: { check: 'body', task: 'Does this text meet the standard?' },
};
const oracle =
	'Locator-limit commission 2026-11-06 and Lead disposition 01a1aee3-6678; nudge Specification R2/C2';

function candidateCount(scenario: LocatorScenario): number {
	return scenario === 'lines254'
		? 254
		: ['over-limit', 'no-locator', 'pass'].includes(scenario)
			? 256
			: scenario === 'lines255'
				? 255
				: 4;
}
export function expectedCriteria(count: number): JsonObject {
	return Object.fromEntries(
		Array.from({ length: count }, (_value, index) => [
			`L${firstCandidateLine + index}`,
			`Candidate ${index + 1}.`,
		]),
	);
}
function findingRoute(name: string): JsonObject {
	return {
		finding: 'The text needs attention.',
		['at']: 'body',
		locate: { name, task: 'Choose the relevant candidate line.', min_probability: 0.7 },
	};
}
export function prepareLocatorFixture(destination: string, scenario: LocatorScenario): string {
	requireCondition(
		!filesystem.existsSync(destination),
		'Locator fixture destination already exists',
	);
	for (const directory of ['documents', 'rulebook', 'evidence/rules'])
		filesystem.mkdirSync(path.join(destination, directory), { recursive: true });
	writeJson(path.join(destination, 'evidence/sgconfig.yml'), { ruleDirs: ['rules'] });
	const noRoute = findingRoute(locatorName);
	if (scenario === 'no-locator') delete noRoute['locate'];
	const steps: JsonObject =
		scenario === 'mechanical'
			? {
					gate: {
						mechanical_check: { tool: 'contains', field: 'body', text: 'NEVER_PRESENT' },
						yes: 'pass',
						['no']: noRoute,
					},
				}
			: {
					gate: {
						ask: 'verdict',
						yes_at_or_above: 0.7,
						no_at_or_below: 0.3,
						yes: findingRoute('alternative_line'),
						['no']: scenario === 'pass' ? 'pass' : noRoute,
					},
				};
	writeJson(path.join(destination, rulesName), {
		K001: {
			name: 'located_subject',
			evidence: 'subject',
			severity: 'warn',
			...(scenario === 'mechanical' ? {} : { questions: { verdict: authoredQuestion } }),
			decision: { start: 'gate', steps },
		},
	});
	filesystem.writeFileSync(
		path.join(destination, rulebookName),
		'The text meets the standard. <!-- K001 -->\n',
	);
	writeJson(path.join(destination, configName), {
		rulebook: [{ markdown: rulebookName, rules: rulesName }],
		marker_prefix: 'K',
		check: { files: ['documents/*.md'] },
		evidence: {
			ast_grep: 'evidence/sgconfig.yml',
			kinds: {
				subject: {
					inline_rule: '{language: Markdown, rule: {kind: paragraph}}',
					fields: { body: { from: 'remainder' } },
				},
			},
		},
		limits: { max_locate_lines: 255, evidence_max_chars: 20000, request_max_chars: 100000 },
		decision_model: { model: 'typesafe/locator-boundary' },
	});
	filesystem.writeFileSync(
		path.join(destination, subjectFile),
		fixtureText(candidateCount(scenario)),
	);
	if (scenario === 'over-limit')
		filesystem.writeFileSync(path.join(destination, peerFile), fixtureText(3));
	return destination;
}

function fixtureText(count: number): string {
	return `# Document\n\n${Array.from({ length: count }, (_value, index) => `Candidate ${index + 1}.`).join('\n')}\n`;
}

function replyFor(body: JsonObject, scenario: LocatorScenario): JsonObject {
	const reply = responseBody(body, 0.1);
	for (const [name, question] of Object.entries(objectValue(body['questions']))) {
		if (name === 'verdict') continue;
		const criteria = objectValue(objectValue(question)['criteria']);
		const selected = 'L5';
		requireCondition(selected in criteria, 'Fixture locator no longer offers the chosen line');
		const probability = scenario === 'low' ? 0.6 : 1;
		objectValue(reply['answers'])[name] = {
			type: 'choice',
			choice: selected,
			confidence: 0.99,
			probabilities: Object.fromEntries(
				Object.keys(criteria).map((label) => [
					label,
					label === selected ? probability : (1 - probability) / (Object.keys(criteria).length - 1),
				]),
			),
		};
	}
	return reply;
}

export function requireLocatorRequests(
	captures: readonly CapturedRequest[],
	scenario: LocatorScenario,
): void {
	equal(
		captures.length,
		scenario === 'mechanical' ? 0 : scenario === 'over-limit' ? 2 : 1,
		'Wrong real request count',
	);
	const seen: string[] = [];
	for (const { body } of captures) {
		const state = objectValue(body['state']);
		const file = textValue(objectValue(state['where'])['file']);
		seen.push(file);
		const questions = objectValue(body['questions']);
		equal(questions['verdict'], authoredQuestion, 'Authored question was changed or dropped');
		const count = file === peerFile ? 3 : candidateCount(scenario);
		const fits = count <= 255;
		const expectedLocators = fits ? [locatorName, 'alternative_line'].toSorted() : [];
		equal(
			Object.keys(questions)
				.filter((name) => name !== 'verdict')
				.toSorted(),
			expectedLocators,
			'Wrong offered locators',
		);
		for (const name of expectedLocators) {
			const question = objectValue(questions[name]);
			equal(question['type'], 'choice', 'Locator primitive changed');
			equal(
				question['criteria'],
				expectedCriteria(count),
				'Locator line criteria changed or truncated',
			);
			equal(
				question['instructions'],
				{ check: 'body', task: 'Choose the relevant candidate line.' },
				'Locator instructions changed',
			);
		}
	}
	if (scenario !== 'mechanical')
		equal(
			seen.toSorted(),
			scenario === 'over-limit' ? [peerFile, subjectFile] : [subjectFile],
			'Wrong piece request identities',
		);
}

export function requireLocatorReport(report: JsonObject, scenario: LocatorScenario): void {
	equal(objectValue(report['status'])['kind'], 'clean', 'Warning-only/pass status changed');
	const results = arrayValue(report['results']).map(objectValue);
	equal(results.length, scenario === 'over-limit' ? 2 : 1, 'Missing or duplicate result');
	const findings = arrayValue(report['findings']).map(objectValue);
	equal(findings.length, scenario === 'pass' ? 0 : results.length, 'Wrong finding count');
	for (const row of results) {
		const file = textValue(row['file']);
		requireCondition(
			file === subjectFile || (scenario === 'over-limit' && file === peerFile),
			'Unexpected result file',
		);
		equal(row['name'], 'located_subject', 'Wrong result rule');
		equal(row['outcome'], scenario === 'pass' ? 'pass' : 'finding', 'Wrong result outcome');
		equal(row['basis'], scenario === 'mechanical' ? 'mechanical' : 'judged', 'Basis changed');
		equal(row['path'], ['gate'], 'Walk history changed');
		const omitted = scenario === 'over-limit' && file === subjectFile;
		if (omitted)
			equal(
				row['locator_omitted'],
				{
					locator: locatorName,
					field: 'body',
					reason: 'line_limit',
					candidate_lines: 256,
					limit: 255,
				},
				'Reached omission evidence differs',
			);
		else
			requireCondition(
				!('locator_omitted' in row),
				'Omission leaked to another piece or unreached route',
			);
		const answers = row['answers'] === undefined ? {} : objectValue(row['answers']);
		equal(
			Object.keys(answers).toSorted(),
			scenario === 'mechanical'
				? []
				: omitted || ['no-locator', 'pass'].includes(scenario)
					? ['verdict']
					: [locatorName, 'verdict'],
			'Unread locator answer was reported',
		);
		if (scenario === 'pass') continue;
		const finding = findings.find((item) => item['file'] === file);
		requireCondition(finding, 'Finding/result join missing');
		const resolved = !omitted && !['low', 'no-locator', 'mechanical'].includes(scenario);
		equal(
			[finding['line'], finding['anchor']],
			[resolved ? 5 : 3, resolved ? 'resolved' : 'default'],
			'Fallback mistaken for observed location',
		);
		if (omitted) equal(finding['locator_omitted'], row['locator_omitted'], 'Finding omission lost');
		else requireCondition(!('locator_omitted' in finding), 'Unrelated finding acquired omission');
	}
	equal(report['incomplete'], [], 'Valid fixture became incomplete');
}

async function observeAdmission(observer: RulebookProcess, root: string): Promise<void> {
	configureEndpoint(root, observer.endpoint);
	await runSequence([254, 255, 256, 1000, 0], async (limit) => {
		configure(root, (config) => {
			objectValue(config['limits'])['max_locate_lines'] = limit;
		});
		const result = await observer.run({
			root,
			name: `check-${limit}`,
			arguments: ['lint', '--config', configName, '--check'],
			expectedExit: limit === 254 || limit === 255 ? 0 : 2,
		});
		equal(result.captures.length, 0, 'Admission dispatched HTTP');
		if (limit > 255)
			requireCondition(
				(result.stdout.toString('utf8') + result.stderr.toString('utf8')).includes(
					'limits.max_locate_lines',
				),
				'Refusal lost exact field path',
			);
		if (limit === 0)
			requireCondition(
				result.stderr.toString('utf8').includes('expected a nonzero'),
				'Zero limit lost its native nonzero refusal',
			);
	});
	configure(root, (config) => {
		delete objectValue(config['limits'])['max_locate_lines'];
	});
	const defaulted = await observer.report({
		root,
		name: 'default-limit',
		arguments: lintArguments('--all'),
		expectedExit: 0,
		credential: true,
	});
	equal(objectValue(defaulted.report['limits'])['max_locate_lines'], 80, 'Default limit changed');
	configure(root, (config) => {
		objectValue(config['limits'])['max_locate_lines'] = 255;
	});
}

export async function runLocatorJourney(options: JourneyArguments): Promise<void> {
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	await runSequence(locatorScenarios, (scenario) => {
		prepareLocatorFixture(path.join(evidence, scenario), scenario);
		return Promise.resolve();
	});
	if (options.prepare) {
		writeJson(path.join(evidence, 'prepared.json'), {
			product_observed: false,
			oracle,
			scenarios: locatorScenarios,
		});
		process.stdout.write('Prepared locator fixtures; product unobserved\n');
		return;
	}
	const target = declaredTarget({
		binary: options.binary ?? '',
		source: options['source-root'],
		commit: options.commit ?? '',
		...(options.sha256 ? { digest: options.sha256 } : {}),
	});
	requireDebugSignature(target.binary);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		observer.reply = (body) => replyFor(body, 'lines254');
		await observeAdmission(observer, path.join(evidence, 'lines254'));
		await runSequence(locatorScenarios, async (scenario) => {
			const root = path.join(evidence, scenario);
			configureEndpoint(root, observer.endpoint);
			observer.reply = (body) => replyFor(body, scenario);
			const result = await observer.report({
				root,
				name: scenario,
				arguments: lintArguments('--all'),
				expectedExit: 0,
				credential: scenario !== 'mechanical',
			});
			requireLocatorRequests(result.captures, scenario);
			requireLocatorReport(result.report, scenario);
			if (scenario === 'over-limit') {
				const sarif = await observer.report({
					root,
					name: 'over-limit-sarif',
					arguments: ['lint', '--config', configName, '--all', '--format', 'sarif'],
					expectedExit: 0,
					credential: true,
				});
				requireLocatorRequests(sarif.captures, scenario);
				const run = objectValue(arrayValue(sarif.report['runs'])[0]);
				const locations = arrayValue(run['results']).map((value) => {
					const finding = objectValue(value);
					const location = objectValue(
						objectValue(arrayValue(finding['locations'])[0])['physicalLocation'],
					);
					return [
						objectValue(location['artifactLocation'])['uri'],
						objectValue(location['region'])['startLine'],
					];
				});
				equal(
					locations,
					[
						[peerFile, 5],
						[subjectFile, 3],
					],
					'SARIF fallback or peer location changed',
				);
				const human = await observer.run({
					root,
					name: 'over-limit-human',
					arguments: ['lint', '--config', configName, '--all'],
					expectedExit: 0,
					credential: true,
				});
				requireLocatorRequests(human.captures, scenario);
				requireCondition(
					human.stdout
						.toString('utf8')
						.includes(
							'locator focus_line omitted: 256 candidate lines exceed limit 255; using default line',
						),
					'Human output hides locator omission',
				);
			}
		});
	} finally {
		await observer.close();
	}
	writeJson(path.join(evidence, 'pass.json'), {
		oracle,
		commit: target.commit,
		binary_sha256: target.digest,
		observed: [
			'config boundaries',
			'wire criteria254/255',
			'reached-result omission256',
			'two-route/two-piece isolation',
			'low/no-locator/pass/mechanical distinctions',
		],
		required_native_joins: [
			'direct SDK256 refusal with count/limit and zeroHTTP',
			'schema agreement',
			'candidate span intersection/count',
			'other request-budget omission reasons',
		],
		unverified: ['provider classification accuracy', 'release/install channels'],
	});
	process.stdout.write('Locator boundary/omission journey passed; inspect required native joins\n');
}

if (import.meta.main)
	try {
		const options = journeyArguments(
			'Locator limits: --prepare writes fixtures; a declared signed candidate proves255-option admission and reached-result omission.',
		);
		if (options) await runLocatorJourney(options);
	} catch (error) {
		process.stderr.write(`Locator observation failed: ${errorMessage(error)}\n`);
		process.exitCode = 2;
	}
