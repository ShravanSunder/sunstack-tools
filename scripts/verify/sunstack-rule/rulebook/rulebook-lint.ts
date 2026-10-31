/** Rulebook pieces, worked decisions, Git scope and peer-skip observation through a frozen CLI. */
import filesystem from 'node:fs';
import path from 'node:path';
import { isDeepStrictEqual } from 'node:util';

import {
	listingPieces,
	location,
	requirePieceContract,
	requireRepeatedRequests,
	requireRequestPiece,
} from './rulebook-evidence-checks.ts';
import {
	arrayValue,
	baselineGit,
	configName,
	configure,
	configureEndpoint,
	equal,
	errorMessage,
	expectedSteps,
	journeyArguments,
	runSequence,
	lintArguments,
	numberValue,
	objectValue,
	oracle,
	prepareFixture,
	readObject,
	requireCondition,
	rulebookName,
	rulesName,
	selectWorkedSkill,
	skillName,
	textValue,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import { declaredTarget, responseBody, RulebookProcess } from './rulebook-process.ts';
import { runSelectionJourney } from '../selection/selection-journey.ts';

export function scriptedReply(body: JsonObject, total = 0.58, confidence = 0.86): JsonObject {
	const reply = responseBody(body);
	if ('kind' in objectValue(body['questions'])) {
		const answers = objectValue(reply['answers']);
		const where = objectValue(objectValue(body['state'])['where']);
		const selected = where['file'] === skillName && where['first_line'] === 49;
		objectValue(answers['covers'])['noul'] = selected
			? total === 0.58
				? 0.3
				: (total - 0.4) / 0.6
			: 0.95;
		if (selected) {
			const choice = objectValue(answers['kind']);
			choice['confidence'] = confidence;
			choice['probabilities'] = {
				outcome: confidence,
				action: (1 - confidence) * 0.7,
				other: (1 - confidence) * 0.3,
			};
		}
	}
	return reply;
}
export function reportStatus(report: JsonObject): string {
	return textValue(objectValue(report['status'])['kind']);
}
export function reportFindings(report: JsonObject): JsonObject[] {
	return arrayValue(report['findings']).map(objectValue);
}
export function findingLine(finding: JsonObject): number {
	return numberValue(finding['line']);
}
function requireWorkedResults(report: JsonObject): void {
	const rows = reportFindings(report);
	requireCondition(
		rows.some(
			(item) =>
				item['marker'] === 'K002' && findingLine(item) === 3 && item['severity'] === 'error',
		),
		'K002 error at line 3 absent',
	);
	const selected = rows.filter((item) => item['marker'] === 'K001' && findingLine(item) === 59);
	const finding = selected[0];
	requireCondition(
		finding &&
			selected.length === 1 &&
			finding['severity'] === 'warn' &&
			finding['field'] === 'completion',
		'K001 warning location/field differs',
	);
	requireCondition(
		Math.abs(numberValue(objectValue(finding['judgment'])['value']) - 0.58) < 1e-12,
		'Combined result was rounded or weighted incorrectly',
	);
	requireCondition(!rows.some((item) => item['marker'] === 'K003'), 'K003 at 0.91 must pass');
}
function requireUncertain(report: JsonObject, step: string): void {
	const selected = arrayValue(report['uncertain'])
		.map(objectValue)
		.filter((item) => item['marker'] === 'K001' && objectValue(item['where'])['first_line'] === 49);
	const item = selected[0];
	requireCondition(item && selected.length === 1, 'K001 uncertain lost');
	equal(item['step'], step, 'Wrong uncertain step');
	equal(
		Object.keys(objectValue(item['answers'])).toSorted(),
		['checkable', 'covers', 'kind'],
		'Uncertain answers incomplete',
	);
	requireCondition(
		Object.keys(objectValue(item['state'])).length > 0,
		'Uncertain sent state absent',
	);
}
export function prepareRulebook(destination: string): void {
	filesystem.mkdirSync(destination);
	for (const name of [
		'all-kinds',
		'worked',
		'uncertain-only',
		'skip',
		'malformed',
		'missing-completion',
		'changes',
		'changed-load',
		'rule-change',
		'untracked',
	]) {
		const root = prepareFixture(
			path.join(destination, name),
			['uncertain-only', 'changes', 'untracked', 'missing-completion'].includes(name)
				? ['K001']
				: [],
		);
		if (
			['worked', 'uncertain-only', 'skip', 'changes', 'rule-change', 'missing-completion'].includes(
				name,
			)
		)
			selectWorkedSkill(root);
		if (name === 'changes')
			configure(root, (config) => {
				const evidence = objectValue(config['evidence']);
				evidence['kinds'] = { step: objectValue(evidence['kinds'])['step'] ?? null };
			});
		if (name === 'missing-completion') {
			const chunks =
				filesystem.readFileSync(path.join(root, skillName), 'utf8').match(/[^\n]*\n|[^\n]+$/g) ??
				[];
			chunks[58] = '\n';
			filesystem.writeFileSync(path.join(root, skillName), chunks.join(''));
			writeJson(path.join(root, 'evidence/rules/selected-step.yml'), {
				id: 'selected-step',
				language: 'Markdown',
				files: ['**/SKILL.md'],
				rule: { all: [{ matches: 'step' }, { regex: '^### 3\\. Choose each observation' }] },
			});
			configure(root, (config) => {
				const evidence = objectValue(config['evidence']);
				const step = objectValue(objectValue(evidence['kinds'])['step']);
				step['rule'] = 'selected-step';
				evidence['kinds'] = { step };
			});
		}
		if (name === 'untracked')
			configure(root, (config) => {
				config['check'] = { files: ['skills/**/SKILL.md'] };
			});
		if (name === 'skip') {
			const entries = readObject(path.join(root, rulesName));
			entries['K004'] = {
				name: 'invalid_peer',
				evidence: 'absent_kind',
				decision: {
					start: 'settled',
					steps: {
						settled: {
							mechanical_check: { tool: 'contains', field: 'body', text: 'x' },
							yes: 'pass',
							['no']: 'pass',
						},
					},
				},
			};
			writeJson(path.join(root, rulesName), entries);
			filesystem.appendFileSync(
				path.join(root, rulebookName),
				'\nAnother standard. <!-- K004 -->\n',
			);
		}
		if (name === 'malformed')
			filesystem.writeFileSync(path.join(root, configName), '{"rulebook": [\n');
	}
	const root = path.join(destination, 'all-kinds');
	for (const file of [
		'verification-design-step-49.json',
		'verification-design-load-51.json',
		'practices-retrospective-step-19.json',
	]) {
		const expected = readObject(path.join(root, 'expected', file));
		const source = filesystem.readFileSync(path.join(root, textValue(expected['file'])));
		equal(
			source
				.subarray(numberValue(expected['start']), numberValue(expected['end']))
				.toString('utf8'),
			expected['text'],
			'S0 expected source bytes differ',
		);
	}
	equal(expectedSteps(root).length, 6, 'Fixture no longer has six steps');
}
function equalSemanticResults(first: JsonObject, second: JsonObject): void {
	for (const key of ['findings', 'uncertain', 'counts', 'exclusions', 'incomplete'])
		equal(first[key], second[key], `Result order/content changed: ${key}`);
}
async function observeChanges(observer: RulebookProcess, fixtures: string): Promise<void> {
	let root = path.join(fixtures, 'changes');
	configureEndpoint(root, observer.endpoint);
	baselineGit(root);
	const original = filesystem.readFileSync(path.join(root, skillName), 'utf8');
	const chunks = original.match(/[^\n]*\n|[^\n]+$/g) ?? [];
	const line = chunks[54];
	requireCondition(line, 'Changed line missing');
	chunks[54] = `${line.replace(/\n$/, '')} Verify this changed obligation.\n`;
	filesystem.writeFileSync(path.join(root, skillName), chunks.join(''));
	const listed = await observer.run({
		root,
		name: 'changed-step',
		arguments: ['lint', '--config', configName, '--evidence', 'step', '--since', 'HEAD'],
		expectedExit: 0,
	});
	const picked = listingPieces(listed.stdout);
	requireCondition(
		picked.length === 1 && picked[0] && location(picked[0])['first_line'] === 49,
		'Changed lines picked another step',
	);
	const first = await observer.report({
		root,
		name: 'changed-step-run',
		arguments: lintArguments('--since', 'HEAD'),
		expectedExit: 0,
		credential: true,
	});
	equal(
		objectValue(first.report['exclusions'])['outside_change_scope'],
		5,
		'Five unchanged steps not excluded',
	);
	equal(first.captures.length, 1, 'Changed scope asked other steps');
	equal(
		objectValue(first.report['usage'])['checked_files_acquired'],
		1,
		'Changed file must count one logical acquisition',
	);
	const repeated = await observer.report({
		root,
		name: 'changed-step-repeat',
		arguments: lintArguments('--since', 'HEAD'),
		expectedExit: 0,
		credential: true,
	});
	requireRepeatedRequests({
		first: first.report,
		second: repeated.report,
		firstBodies: first.captures.map((item) => item.raw),
		secondBodies: repeated.captures.map((item) => item.raw),
	});
	equalSemanticResults(first.report, repeated.report);
	root = path.join(fixtures, 'changed-load');
	configureEndpoint(root, observer.endpoint);
	baselineGit(root);
	filesystem.appendFileSync(
		path.join(root, 'skills/verification-design/references/choosing-the-observation.md'),
		'\nChanged loaded reference.\n',
	);
	const loads = await observer.run({
		root,
		name: 'changed-load',
		arguments: ['lint', '--config', configName, '--evidence', 'load', '--since', 'HEAD'],
		expectedExit: 0,
	});
	requireCondition(
		listingPieces(loads.stdout).some(
			(piece) => location(piece)['first_line'] === 51 && location(piece)['file'] === skillName,
		),
		'Changed target did not pick its load',
	);
	root = path.join(fixtures, 'rule-change');
	configureEndpoint(root, observer.endpoint);
	baselineGit(root);
	filesystem.appendFileSync(path.join(root, rulesName), '\n');
	const widened = await observer.report({
		root,
		name: 'rule-change-widens',
		arguments: lintArguments('--since', 'HEAD'),
		expectedExit: 1,
		credential: true,
	});
	requireCondition(objectValue(widened.report['scope'])['widened'], 'Rule edit did not widen');
	root = path.join(fixtures, 'untracked');
	configureEndpoint(root, observer.endpoint);
	baselineGit(root);
	filesystem.mkdirSync(path.join(root, 'skills/new-skill'));
	filesystem.writeFileSync(path.join(root, 'skills/new-skill/SKILL.md'), original);
	const added = await observer.run({
		root,
		name: 'untracked-whole',
		arguments: ['lint', '--config', configName, '--evidence', 'step'],
		expectedExit: 0,
	});
	const rows = listingPieces(added.stdout);
	requireCondition(
		rows.length === 6 &&
			rows.every((piece) => location(piece)['file'] === 'skills/new-skill/SKILL.md'),
		'Untracked skill not wholly picked',
	);
}
async function observe(observer: RulebookProcess, fixtures: string): Promise<void> {
	const root = path.join(fixtures, 'all-kinds');
	configureEndpoint(root, observer.endpoint);
	const listing = await observer.run({
		root,
		name: 'evidence-all',
		arguments: ['lint', '--config', configName, '--evidence', '--all'],
		expectedExit: 0,
	});
	equal(listing.captures.length, 0, 'Evidence listing dispatched');
	const pieces = listingPieces(listing.stdout);
	requirePieceContract(pieces, root);
	await runSequence(
		Object.keys(
			objectValue(objectValue(readObject(path.join(root, configName))['evidence'])['kinds']),
		),
		async (kind) => {
			const filtered = await observer.run({
				root,
				name: `evidence-${kind}`,
				arguments: ['lint', '--config', configName, '--evidence', kind, '--all'],
				expectedExit: 0,
			});
			equal(filtered.captures.length, 0, 'Kind listing dispatched');
			equal(
				listingPieces(filtered.stdout),
				pieces.filter((piece) => piece['kind'] === kind),
				'Kind filter differs',
			);
		},
	);
	const worked = path.join(fixtures, 'worked');
	configureEndpoint(worked, observer.endpoint);
	observer.reply = scriptedReply;
	const baseline = await observer.report({
		root: worked,
		name: 'worked',
		arguments: lintArguments('--all'),
		expectedExit: 1,
		credential: true,
	});
	equal(reportStatus(baseline.report), 'findings', 'Wrong full status');
	requireWorkedResults(baseline.report);
	equal(baseline.captures.length, 7, 'Worked examples must ask six steps and one description');
	const repeated = await observer.report({
		root: worked,
		name: 'worked-repeat',
		arguments: lintArguments('--all'),
		expectedExit: 1,
		credential: true,
	});
	requireRepeatedRequests({
		first: baseline.report,
		second: repeated.report,
		firstBodies: baseline.captures.map((item) => item.raw),
		secondBodies: repeated.captures.map((item) => item.raw),
	});
	equalSemanticResults(baseline.report, repeated.report);
	const entries = readObject(path.join(worked, rulesName));
	const standards: Record<string, string> = {
		K001: 'Each workflow step states a checkable completion covering its obligations.',
		K003: 'A skill description names when to use the skill.',
	};
	for (const capture of baseline.captures) {
		const questions = objectValue(capture.body['questions']);
		const marker = 'kind' in questions ? 'K001' : 'K003';
		equal(questions, objectValue(entries[marker])['questions'], 'Questions changed on wire');
		const state = objectValue(capture.body['state']);
		equal(objectValue(state['where'])['file'], skillName, 'Asked unselected file');
		const kind = marker === 'K001' ? 'step' : 'skill';
		const matching = pieces.filter(
			(piece) => piece['kind'] === kind && isDeepStrictEqual(piece['where'], state['where']),
		);
		const piece = matching[0];
		const standard = standards[marker];
		requireCondition(
			piece && matching.length === 1 && standard,
			'Request does not identify one listed piece',
		);
		requireRequestPiece({ state, piece, standard, questions });
	}
	await runSequence(
		[
			{ name: 'quality', total: 0.65, confidence: 0.86, step: 'quality' },
			{ name: 'choice', total: 0.58, confidence: 0.55, step: 'kind' },
		],
		async (setting) => {
			observer.reply = (body) => scriptedReply(body, setting.total, setting.confidence);
			const isolated = path.join(fixtures, 'uncertain-only');
			configureEndpoint(isolated, observer.endpoint);
			const result = await observer.report({
				root: isolated,
				name: `uncertain-${setting.name}`,
				arguments: lintArguments('--all'),
				expectedExit: 5,
				credential: true,
			});
			equal(reportStatus(result.report), 'uncertain', 'Wrong uncertain-only status');
			requireUncertain(result.report, setting.step);
		},
	);
	observer.reply = (body) => scriptedReply(body, 0.65);
	const uncertain = await observer.report({
		root: worked,
		name: 'uncertain-plus-error',
		arguments: lintArguments('--all'),
		expectedExit: 1,
		credential: true,
	});
	equal(reportStatus(uncertain.report), 'findings', 'Uncertain outranks error');
	requireUncertain(uncertain.report, 'quality');
	observer.reply = scriptedReply;
	const missing = path.join(fixtures, 'missing-completion');
	configureEndpoint(missing, observer.endpoint);
	const mechanical = await observer.report({
		root: missing,
		name: 'missing-completion',
		arguments: lintArguments('--all'),
		expectedExit: 0,
		credential: true,
	});
	const rows = reportFindings(mechanical.report);
	const finding = rows[0];
	requireCondition(
		mechanical.captures.length === 0 &&
			finding &&
			rows.length === 1 &&
			finding['marker'] === 'K001' &&
			findingLine(finding) === 49 &&
			finding['judgment'] === null,
		'Missing completion must settle mechanically at line49',
	);
	const skipped = path.join(fixtures, 'skip');
	configureEndpoint(skipped, observer.endpoint);
	const partial = await observer.report({
		root: skipped,
		name: 'skip',
		arguments: lintArguments('--all'),
		expectedExit: 3,
		credential: true,
	});
	equal(reportStatus(partial.report), 'incomplete', 'Skip lost incomplete status');
	equal(partial.report['findings'], baseline.report['findings'], 'Bad entry changed peer findings');
	requireCondition(arrayValue(partial.report['skipped_rules']).length > 0, 'Skipped entry absent');
	requireCondition(
		arrayValue(partial.report['incomplete']).some(
			(item) => objectValue(objectValue(item)['reason'])['kind'] === 'coverage_partial',
		),
		'Coverage gap absent',
	);
	const malformed = await observer.report({
		root: path.join(fixtures, 'malformed'),
		name: 'malformed',
		arguments: lintArguments('--all'),
		expectedExit: 2,
	});
	requireCondition(
		reportStatus(malformed.report) === 'invalid' &&
			malformed.captures.length === 0 &&
			objectValue(malformed.report['usage'])['checked_files_acquired'] === 0,
		'Malformed config acquired checked content',
	);
	await observeChanges(observer, fixtures);
}
export async function main(): Promise<void> {
	const options = journeyArguments(
		'Rulebook lint: independently checked pieces, worked decisions, Git scope and peer skips.',
	);
	if (!options) return;
	if (options.case.includes('selection')) {
		await runSelectionJourney(options);
		return;
	}
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
	prepareRulebook(path.join(evidence, 'fixtures'));
	if (options.prepare) {
		writeJson(path.join(evidence, 'prepared.json'), {
			product_observed: false,
			oracle,
			fixture_cases: 10,
		});
		process.stdout.write('Prepared rulebook journey fixtures; product unobserved\n');
		return;
	}
	requireCondition(target, 'Declared target missing');
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		await observe(observer, path.join(evidence, 'fixtures'));
	} finally {
		await observer.close();
	}
	writeJson(path.join(evidence, 'pass.json'), {
		commit: target.commit,
		binary_sha256: target.digest,
		oracle,
		required_joins: [
			'U1/U2/U3 native evidence tests',
			'U5 walk/edge tables',
			'U6 exact body cap cases',
			'U7/U12 acquisition gates; U9 structural/FIFO',
			'U11 source review',
			'hosted inference unobserved',
		],
	});
	process.stdout.write('Rulebook binary journeys passed; inspect required native/hosted joins\n');
}
if (import.meta.main)
	try {
		await main();
	} catch (error) {
		process.stderr.write(`Rulebook lint observation failed: ${errorMessage(error)}\n`);
		process.exitCode = 2;
	}
