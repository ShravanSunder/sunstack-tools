/** S1 --case nudge: real lint JSON/SARIF, exact source-derived states and scripted replies. */
import filesystem from 'node:fs';
import path from 'node:path';

import { requireDebugSignature } from './rulebook-amendment.ts';
import {
	arrayValue,
	configName,
	configureEndpoint,
	equal,
	objectValue,
	readObject,
	requireCondition,
	runSequence,
	rulesName,
	textValue,
	writeJson,
	type JourneyArguments,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import {
	locatedFile,
	locationBody,
	locationQuestions,
	nudgeOracle,
	nudgeState,
	onlyFiles,
	pieceWhere,
	selectedFixture,
	setLocateLimit,
	sourceSpan,
	sourceText,
	standardPlace,
} from './rulebook-nudge-fixtures.ts';
import {
	declaredTarget,
	responseBody,
	RulebookProcess,
	type CapturedRequest,
} from './rulebook-process.ts';

function nudgeReply(body: JsonObject, mode = 'resolved'): JsonObject {
	const reply = responseBody(body);
	const answers = objectValue(reply['answers']);
	const file = textValue(objectValue(objectValue(body['state'])['where'])['file']);
	if (answers['gate']) objectValue(answers['gate'])['noul'] = file.endsWith('/drop.md') ? 0.1 : 0.9;
	if (answers['anchor_line']) {
		const labels = Object.keys(
			objectValue(objectValue(objectValue(body['questions'])['anchor_line'])['criteria']),
		);
		const selected = mode === 'invalid' ? 'L999' : 'L17';
		const probability = mode === 'low' ? 0.6 : 0.9;
		answers['anchor_line'] = {
			type: 'choice',
			choice: selected,
			confidence: probability,
			probabilities: Object.fromEntries(
				labels.map((label) => [
					label,
					label === 'L17' ? probability : (1 - probability) / (labels.length - 1),
				]),
			),
		};
	}
	return reply;
}
function normalizedAnswers(body: JsonObject, mode = 'resolved'): JsonObject {
	const wire = objectValue(nudgeReply(body, mode)['answers']);
	const answers: JsonObject = {};
	if (wire['gate'])
		answers['gate'] = { type: 'noul', yes_probability: objectValue(wire['gate'])['noul'] ?? null };
	if (wire['anchor_line']) {
		const choice = objectValue(wire['anchor_line']);
		answers['anchor_line'] = {
			type: 'choice',
			selected_label: choice['choice'] ?? null,
			confidence: choice['confidence'] ?? null,
			probabilities: choice['probabilities'] ?? null,
		};
	}
	return answers;
}
function expectedRow(
	root: string,
	marker: string,
	file: string,
	outcome: string,
	steps: string[],
	basis?: string,
): JsonObject {
	const entry = objectValue(readObject(path.join(root, rulesName))[marker]);
	const where = pieceWhere(
		root,
		file,
		marker === 'K001' || marker === 'K005'
			? Buffer.byteLength(sourceText(root, file).split('\n').slice(0, 5).join('\n') + '\n')
			: 0,
	);
	return {
		name: entry['name'] ?? null,
		marker,
		file,
		first_line: where['first_line'] ?? null,
		last_line: where['last_line'] ?? null,
		outcome,
		step: steps.at(-1) ?? null,
		path: steps,
		...(basis ? { basis } : {}),
	};
}
function requireOneRow(report: JsonObject, expected: JsonObject): void {
	equal(report['results'], [expected], 'Exact nudge result row differs from Specification');
}
function requireCaptures(
	root: string,
	marker: string,
	captures: CapturedRequest[],
	offer = true,
): void {
	const entry = objectValue(readObject(path.join(root, rulesName))[marker]);
	for (const capture of captures) {
		const file = textValue(objectValue(objectValue(capture.body['state'])['where'])['file']);
		const questions =
			marker === 'K001' ? locationQuestions(root, offer) : objectValue(entry['questions']);
		equal(
			capture.body,
			{ model: 'jev-latest', state: nudgeState(root, marker, file), questions },
			'Exact sent body differs from fixture oracle',
		);
	}
}
function relatedPlaces(root: string): JsonObject[] {
	const state = nudgeState(root, 'K001', locatedFile);
	const places = [standardPlace(root, 'K001')];
	for (const value of arrayValue(state['targets'])) {
		const target = objectValue(value);
		if (typeof target['opening'] !== 'string' || target['opening'].length === 0) continue;
		const opening = target['opening'];
		places.push({
			label: 'targets',
			file: target['path'] ?? null,
			first_line: 1,
			last_line: opening.replace(/\n$/, '').split('\n').length,
		});
	}
	// The first-line field ends at the next line's column 1; its last touched line is 6.
	places.push({ label: 'heading', file: locatedFile, first_line: 6, last_line: 6 });
	return places;
}
function requireLocated(root: string, report: JsonObject, resolved: boolean): void {
	const findings = arrayValue(report['findings']).map(objectValue);
	equal(findings.length, 1, 'Located finding count');
	const finding = findings[0];
	requireCondition(finding, 'No located finding');
	equal(
		[finding['line'], finding['anchor'], finding['basis']],
		[resolved ? 17 : 7, resolved ? 'resolved' : 'default', 'judged'],
		'Anchor or basis differs',
	);
	equal(finding['related'], relatedPlaces(root), 'Related order/files/ranges differ');
	const text = sourceText(root, locatedFile);
	const start = Buffer.byteLength(text.split('\n').slice(0, 6).join('\n') + '\n');
	equal(
		finding['span'],
		sourceSpan(root, locatedFile, start, start + Buffer.byteLength(locationBody(root))),
		'Locator narrowed or moved field span',
	);
}
async function observeLocation(observer: RulebookProcess, fixtures: string): Promise<void> {
	await runSequence(['resolved', 'low', 'over-limit', 'invalid'], async (mode) => {
		const root = path.join(fixtures, mode);
		configureEndpoint(root, observer.endpoint);
		observer.reply = (body) => nudgeReply(body, mode);
		const observed = await observer.report({
			root,
			name: mode,
			arguments: ['lint', '--config', configName, '--all', '--format', 'json'],
			expectedExit: mode === 'invalid' ? 3 : 0,
			credential: true,
		});
		equal(observed.captures.length, 1, 'Located route must make exactly one request');
		requireCaptures(root, 'K001', observed.captures, mode !== 'over-limit');
		if (mode === 'invalid') {
			equal(arrayValue(observed.report['findings']), [], 'Invalid Choice became a finding');
			requireCondition(
				arrayValue(observed.report['incomplete']).some(
					(part) => objectValue(objectValue(part)['reason'])['kind'] === 'invalid_response',
				),
				'Invalid Choice not refused by real validator',
			);
			return;
		}
		requireLocated(root, observed.report, mode === 'resolved');
		const capture = observed.captures[0];
		requireCondition(capture, 'Missing captured body');
		requireOneRow(observed.report, {
			...expectedRow(root, 'K001', locatedFile, 'finding', ['gate'], 'judged'),
			answers: normalizedAnswers(capture.body, mode),
		});
		if (mode === 'resolved') {
			const sarif = await observer.report({
				root,
				name: 'resolved-sarif',
				arguments: ['lint', '--config', configName, '--all', '--format', 'sarif'],
				expectedExit: 0,
				credential: true,
			});
			const run = objectValue(arrayValue(sarif.report['runs'])[0]);
			const result = objectValue(arrayValue(run['results'])[0]);
			const physical = objectValue(
				objectValue(arrayValue(result['locations'])[0])['physicalLocation'],
			);
			equal(objectValue(physical['region'])['startLine'], 17, 'SARIF uses the old span line');
			equal(
				arrayValue(result['relatedLocations']).map((item) => {
					const value = objectValue(item);
					const location = objectValue(value['physicalLocation']);
					const region = objectValue(location['region']);
					return {
						label: objectValue(value['message'])['text'],
						file: objectValue(location['artifactLocation'])['uri'],
						first_line: region['startLine'],
						last_line: region['endLine'],
					};
				}),
				relatedPlaces(root),
				'SARIF related places differ',
			);
			requireCaptures(root, 'K001', sarif.captures);
			equal(
				sarif.captures.map((item) => item.raw),
				observed.captures.map((item) => item.raw),
				'JSON/SARIF exact request bytes differ',
			);
			await observer.report({
				root,
				name: 'warning-limit',
				arguments: [
					'lint',
					'--config',
					configName,
					'--all',
					'--format',
					'json',
					'--max-warnings',
					'0',
				],
				expectedExit: 1,
				credential: true,
			});
		}
	});
}
async function observeHistory(observer: RulebookProcess, fixtures: string): Promise<void> {
	observer.reply = (body) => nudgeReply(body);
	await runSequence(
		[
			['drop', 'K002', 'cases/drop.md', 'not_applicable', ['gate'], 'judged', 0],
			['pass', 'K002', 'cases/pass.md', 'pass', ['gate'], 'judged', 0],
			['missing', 'K003', 'cases/missing.md', 'incomplete', ['gate', 'missing'], undefined, 3],
			['judged', 'K004', 'cases/judged.md', 'finding', ['gate', 'mechanical'], 'judged', 1],
		] satisfies [string, string, string, string, string[], string | undefined, number][],
		async ([name, marker, file, outcome, steps, basis, exit]) => {
			const root = path.join(fixtures, name);
			configureEndpoint(root, observer.endpoint);
			const observed = await observer.report({
				root,
				name: `history-${name}`,
				arguments: ['lint', '--config', configName, '--all', '--format', 'json'],
				expectedExit: exit,
				credential: true,
			});
			equal(observed.captures.length, 1, 'History case request count');
			requireCaptures(root, marker, observed.captures);
			requireOneRow(observed.report, {
				...expectedRow(root, marker, file, outcome, steps, basis),
				answers: { gate: { type: 'noul', yes_probability: name === 'drop' ? 0.1 : 0.9 } },
			});
			if (name === 'judged') {
				const finding = objectValue(arrayValue(observed.report['findings'])[0]);
				equal(
					[finding['severity'], finding['basis']],
					['error', 'judged'],
					'Authored judged error changed',
				);
			}
		},
	);
}
async function observeControls(observer: RulebookProcess, fixtures: string): Promise<void> {
	await runSequence(
		[
			['mechanical', 'K005', locatedFile, 0],
			['route-error', 'K008', 'cases/judged.md', 1],
		] satisfies [string, string, string, number][],
		async ([name, marker, file, exit]) => {
			const root = path.join(fixtures, name);
			configureEndpoint(root, observer.endpoint);
			const observed = await observer.report({
				root,
				name,
				arguments: ['lint', '--config', configName, '--all', '--format', 'json'],
				expectedExit: exit,
			});
			equal(observed.captures.length, 0, 'Mechanical settle dispatched');
			requireOneRow(
				observed.report,
				expectedRow(root, marker, file, 'finding', ['mechanical'], 'mechanical'),
			);
			const finding = objectValue(arrayValue(observed.report['findings'])[0]);
			equal(finding['anchor'], 'default', 'Mechanical route anchor');
			equal(
				finding['related'],
				[standardPlace(root, marker)],
				'Rule without related fields must list only its standard',
			);
			if (name === 'route-error')
				equal(finding['severity'], 'error', 'Route failed to override warn');
			else equal(finding['line'], 7, 'Mechanical located route changed default line');
		},
	);
	await runSequence(['long', 'short'], async (size) => {
		const root = path.join(fixtures, size);
		configureEndpoint(root, observer.endpoint);
		const observed = await observer.report({
			root,
			name: `cut-${size}`,
			arguments: ['lint', '--config', configName, '--all', '--format', 'json'],
			expectedExit: 0,
			credential: true,
		});
		equal(observed.captures.length, 1, 'Cut fixture did not dispatch');
		requireCaptures(root, 'K006', observed.captures);
		requireOneRow(observed.report, {
			...expectedRow(root, 'K006', `cases/${size}.md`, 'finding', ['gate'], 'judged'),
			answers: { gate: { type: 'noul', yes_probability: 0.9 } },
			...(size === 'long' ? { cut: ['body'] } : {}),
		});
		equal(
			Object.values(objectValue(observed.report['evidence_seen'])),
			[nudgeState(root, 'K006', `cases/${size}.md`)],
			'Report evidence lacks exact cut/authored state',
		);
	});
	const root = path.join(fixtures, 'mixed');
	configureEndpoint(root, observer.endpoint);
	const mixed = await observer.report({
		root,
		name: 'mixed-json',
		arguments: ['lint', '--config', configName, '--all', '--format', 'json'],
		expectedExit: 0,
	});
	const findings = arrayValue(mixed.report['findings']).map(objectValue);
	equal(findings.length, 2, 'Mixed Markdown/JSON findings absent');
	const json = findings.find((finding) => finding['marker'] === 'K007');
	requireCondition(json, 'JSON finding absent');
	const questionLine =
		sourceText(root, 'data/rules.json')
			.split('\n')
			.findIndex((line) => line.includes('"missing_question"')) + 1;
	equal(
		[json['file'], json['line']],
		['data/rules.json', questionLine],
		'JSON question location differs',
	);
	const broken = path.join(fixtures, 'broken-json');
	configureEndpoint(broken, observer.endpoint);
	const isolated = await observer.report({
		root: broken,
		name: 'broken-json',
		arguments: ['lint', '--config', configName, '--all', '--format', 'json'],
		expectedExit: 3,
	});
	const valid = arrayValue(isolated.report['findings']).map(objectValue);
	requireCondition(
		valid.some((finding) => finding['file'] === 'data/rules.json'),
		'Broken JSON suppressed valid peer',
	);
	requireCondition(
		arrayValue(isolated.report['incomplete']).some(
			(part) => objectValue(part)['file'] === 'data/broken.json',
		),
		'Broken JSON gap absent',
	);
}
export async function runNudgeJourney(options: JourneyArguments): Promise<void> {
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
	const fixtures = path.join(evidence, 'fixtures');
	filesystem.mkdirSync(fixtures);
	for (const name of ['resolved', 'low', 'over-limit', 'invalid']) {
		const root = selectedFixture(path.join(fixtures, name), ['K001']);
		onlyFiles(root, [locatedFile]);
		if (name === 'over-limit') setLocateLimit(root, 1);
		locationBody(root);
	}
	for (const [name, marker, file] of [
		['drop', 'K002', 'cases/drop.md'],
		['pass', 'K002', 'cases/pass.md'],
		['missing', 'K003', 'cases/missing.md'],
		['judged', 'K004', 'cases/judged.md'],
		['mechanical', 'K005', locatedFile],
		['route-error', 'K008', 'cases/judged.md'],
		['long', 'K006', 'cases/long.md'],
		['short', 'K006', 'cases/short.md'],
	] satisfies [string, string, string][]) {
		const root = selectedFixture(path.join(fixtures, name), [marker]);
		onlyFiles(root, [file]);
	}
	for (const name of ['mixed', 'broken-json']) {
		const root = selectedFixture(path.join(fixtures, name), ['K005', 'K007']);
		onlyFiles(root, [locatedFile, 'data/*.json']);
		if (name === 'broken-json')
			filesystem.writeFileSync(path.join(root, 'data/broken.json'), '{"missing_question":\n');
	}
	if (!target) {
		writeJson(path.join(evidence, 'prepared.json'), {
			product_observed: false,
			oracle: nudgeOracle,
			cases: filesystem.readdirSync(fixtures),
		});
		return;
	}
	requireDebugSignature(target.binary);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		await runSequence(filesystem.readdirSync(fixtures), async (name) => {
			const root = path.join(fixtures, name);
			configureEndpoint(root, observer.endpoint);
			const ready = await observer.run({
				root,
				name: `ready-${name}`,
				arguments: ['lint', '--config', configName, '--check'],
				expectedExit: 0,
			});
			equal(ready.captures.length, 0, 'Readiness check dispatched');
		});
		await observeLocation(observer, fixtures);
		await observeHistory(observer, fixtures);
		await observeControls(observer, fixtures);
	} finally {
		await observer.close();
	}
	writeJson(path.join(evidence, 'pass.json'), {
		target,
		oracle: nudgeOracle,
		supported: ['S1 U1/U2/U3/U4/U5/U8/U9'],
		required_joins: [
			'S2 admission',
			'S3 focused tests and schema',
			'U10 legacy projection/diff',
			'Lead S4 real model',
		],
	});
	process.stdout.write(
		'Nudge CLI journey passed; native, compatibility and keyed joins remain separate\n',
	);
}
