/** Filters OD: real lint over isolated folders, with only Choice replies controlled. */
import filesystem from 'node:fs';
import path from 'node:path';

import { requireDebugSignature } from './rulebook-amendment.ts';
import {
	arrayValue,
	configName,
	configure,
	configureEndpoint,
	equal,
	lintArguments,
	objectValue,
	parseObject,
	readObject,
	requireCondition,
	runSequence,
	rulebookName,
	rulesName,
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

export const filtersOracle =
	'dd92e2c:docs/specs/2026-11-04-owner-assessment/verification-design.md';
const filtersProgramDesign =
	'906fc9ed83f9bd1b7bb4c59b82b891a960d4343a:docs/specs/2026-11-04-owner-assessment/program-design.md';
export const includedFolder = 'plugins/one/skills/review';
export const excludedFolder = 'plugins/two/skills/review';
export const includedMembers = ['one.md', 'two.md', 'three.md'].map(
	(name) => `${includedFolder}/notes/${name}`,
);
export const excludedMember = `${excludedFolder}/notes/one.md`;
export const folderlessMember = 'elsewhere/unowned.md';
export const filterQuestion: JsonObject = {
	type: 'choice',
	instructions: { task: 'Classify this description for the fixture rules.' },
	criteria: { eligible: 'General instructions.', blocked: 'Specific instructions.' },
};
const descriptions = new Map([
	[includedFolder, 'General instructions for an included folder.'],
	[excludedFolder, 'Specific instructions for an excluded folder.'],
]);
export type FilterScenario = 'judged' | 'unsure' | 'failed' | 'folderless' | 'member';

/** Authored fixture values are the oracle; no expectation is read from product output. */
export function prepareFilterFixture(destination: string): string {
	requireCondition(!filesystem.existsSync(destination), 'Filters fixture already exists');
	for (const directory of ['rulebook', 'evidence/rules', 'elsewhere'])
		filesystem.mkdirSync(path.join(destination, directory), { recursive: true });
	writeJson(path.join(destination, 'evidence/sgconfig.yml'), { ruleDirs: ['rules'] });
	writeJson(path.join(destination, rulesName), {
		K001: {
			name: 'mechanical_gate',
			evidence: 'subject',
			only_when: { category: ['eligible'] },
			decision: {
				start: 'approved',
				steps: {
					approved: {
						mechanical_check: { tool: 'contains', field: 'body', text: 'Approved' },
						yes: 'pass',
						['no']: { finding: 'Missing approval.' },
					},
				},
			},
		},
		K002: {
			name: 'asking_gate',
			evidence: 'subject',
			only_when: { category: ['eligible'] },
			questions: {
				verdict: {
					type: 'noul',
					instructions: { check: 'body', task: 'Is this instruction approved?' },
				},
			},
			decision: {
				start: 'approved',
				steps: {
					approved: {
						ask: 'verdict',
						yes_at_or_above: 0.7,
						no_at_or_below: 0.3,
						yes: 'pass',
						['no']: 'not_applicable',
					},
				},
			},
		},
	});
	filesystem.writeFileSync(
		path.join(destination, rulebookName),
		'Instructions have approval. <!-- K001 -->\n\nInstructions are judged. <!-- K002 -->\n',
	);
	writeJson(path.join(destination, configName), {
		rulebook: [{ markdown: rulebookName, rules: rulesName }],
		marker_prefix: 'K',
		check: { files: ['plugins/**/notes/*.md', 'elsewhere/*.md'] },
		evidence: {
			ast_grep: 'evidence/sgconfig.yml',
			kinds: {
				description: {
					inline_rule:
						'{language: Markdown, files: ["**/overview.md"], rule: {matches: frontmatter}}',
					fields: { description: { from: 'yaml', key: 'description' } },
				},
				subject: {
					inline_rule: '{language: Markdown, rule: {kind: paragraph}}',
					fields: { body: { from: 'remainder' } },
				},
			},
		},
		filters: {
			category: {
				per: 'plugins/*/skills/{name}',
				reads: { file: 'overview.md', kind: 'description', field: 'description' },
				question: filterQuestion,
				min_probability: 0.7,
			},
		},
		decision_model: { model: 'typesafe/requested-filters' },
	});
	for (const [folder, description] of descriptions) {
		filesystem.mkdirSync(path.join(destination, folder, 'notes'), { recursive: true });
		filesystem.writeFileSync(
			path.join(destination, folder, 'overview.md'),
			`---\ndescription: ${description}\n---\n\nCONTEXT_ONLY_SENTINEL.\n`,
		);
	}
	for (const member of includedMembers)
		filesystem.writeFileSync(path.join(destination, member), 'Approved instruction.\n');
	filesystem.writeFileSync(
		path.join(destination, excludedMember),
		'Approved first instruction.\n\nApproved second instruction.\n\nApproved third instruction.\n',
	);
	return destination;
}

export function filterReply(body: JsonObject, scenario: FilterScenario): JsonObject {
	const reply = responseBody(body, 0.95);
	const state = objectValue(body['state']);
	const where = objectValue(state['where']);
	if (!('folder' in where)) return reply;
	const folder = textValue(where['folder']);
	requireCondition(descriptions.has(folder), 'Filter asked about an unexpected folder');
	const excluded = folder === excludedFolder;
	const selected = excluded && scenario !== 'unsure' ? 'blocked' : 'eligible';
	const probabilities = !excluded
		? { eligible: 0.7, blocked: 0.3 }
		: scenario === 'unsure'
			? { eligible: 0.3, blocked: 0.7 }
			: { eligible: 0.1, blocked: 0.9 };
	const questions = Object.keys(objectValue(body['questions']));
	equal(questions.length, 1, 'Filter must ask one Choice');
	const question = questions[0];
	requireCondition(question, 'Filter question missing');
	objectValue(reply['answers'])[question] = {
		type: 'choice',
		choice: selected,
		confidence: 0.99,
		probabilities,
	};
	return reply;
}

export function requireFilterRequests(
	captures: readonly CapturedRequest[],
	scenario: FilterScenario,
): void {
	const folders: string[] = [];
	const ruleFiles: string[] = [];
	for (const { body } of captures) {
		const state = objectValue(body['state']);
		const where = objectValue(state['where']);
		if ('folder' in where) {
			const folder = textValue(where['folder']);
			folders.push(folder);
			equal(Object.values(objectValue(body['questions'])), [filterQuestion], 'Choice changed');
			equal(
				state,
				{ where: { folder }, description: descriptions.get(folder) },
				'Description-only filter state changed',
			);
		} else {
			ruleFiles.push(textValue(where['file']));
			equal(Object.keys(objectValue(body['questions'])), ['verdict'], 'Unexpected rule ask');
		}
	}
	const expectedFolders =
		scenario === 'folderless'
			? []
			: scenario === 'member'
				? [includedFolder]
				: [includedFolder, excludedFolder];
	equal(folders.toSorted(), expectedFolders.toSorted(), 'Wrong filter identities/count');
	equal(
		ruleFiles.toSorted(),
		scenario === 'folderless'
			? []
			: scenario === 'member'
				? includedMembers.slice(0, 1)
				: includedMembers.toSorted(),
		'Excluded/context members asked, or eligible members omitted',
	);
}

export function requireFilterReport(report: JsonObject, scenario: FilterScenario): void {
	const status =
		scenario === 'unsure'
			? 'uncertain'
			: scenario === 'failed' || scenario === 'folderless'
				? 'incomplete'
				: 'clean';
	equal(objectValue(report['status'])['kind'], status, 'Wrong filter status');
	equal(report['findings'], [], 'Filter fixture must not invent findings');
	equal(report['uncertain'], [], 'Filter uncertainty is not a rule result');
	equal(report['counts'], { error: 0, warn: 0 }, 'Filter status fabricated a finding');
	equal(
		objectValue(report['exclusions'])['not_applicable'],
		scenario === 'judged' ? 6 : 0,
		'Skips and exclusions were conflated',
	);
	const entries = arrayValue(report['filter_answers']).map(objectValue);
	const expectedFolders =
		scenario === 'folderless'
			? []
			: scenario === 'member'
				? [includedFolder]
				: [includedFolder, excludedFolder];
	equal(
		entries.map((entry) => textValue(entry['folder'])).toSorted(),
		expectedFolders.toSorted(),
		'Answers lost/repeated',
	);
	for (const entry of entries) {
		const excluded = entry['folder'] === excludedFolder;
		equal(entry['filter'], 'category', 'Wrong filter identity');
		equal(entry['requested_model'], 'typesafe/requested-filters', 'Requested model lost');
		equal(entry['input_cut'], false, 'Short description spuriously cut');
		if (excluded && scenario === 'failed') {
			equal(entry['state'], 'incomplete', 'Failed filter became an answer');
			equal(entry['resolved_model'], null, 'Failed request fabricated a responding model');
			const reason = objectValue(entry['reason']);
			equal(reason['kind'], 'request', 'Failure reason lost');
			equal(objectValue(reason['reason'])['kind'], 'provider_failure', 'Provider failure lost');
			equal(objectValue(reason['reason'])['http_status'], 400, 'Wrong HTTP failure');
		} else {
			equal(entry['resolved_model'], 'typesafe/controlled-rulebook', 'Resolved model lost');
			equal(
				entry['state'],
				excluded && scenario === 'unsure' ? 'unsure' : 'judged',
				'Wrong certainty',
			);
			equal(
				entry['probabilities'],
				!excluded
					? { eligible: 0.7, blocked: 0.3 }
					: scenario === 'unsure'
						? { eligible: 0.3, blocked: 0.7 }
						: { eligible: 0.1, blocked: 0.9 },
				'Probabilities lost',
			);
			equal(
				entry['answer'],
				excluded && scenario !== 'unsure' ? 'blocked' : 'eligible',
				'Selected answer replaced',
			);
			equal(
				entry['probability'],
				!excluded ? 0.7 : scenario === 'unsure' ? 0.3 : 0.9,
				'Wrong selected probability',
			);
		}
	}
	const rows = arrayValue(report['results']).map(objectValue);
	const expectedRows: unknown[] = [];
	for (const [name, marker] of [
		['mechanical_gate', 'K001'],
		['asking_gate', 'K002'],
	] as const) {
		if (scenario !== 'folderless')
			for (const file of scenario === 'member'
				? includedMembers.slice(0, 1)
				: includedMembers.toSorted())
				expectedRows.push([
					name,
					marker,
					file,
					1,
					'pass',
					marker === 'K001' ? 'mechanical' : 'judged',
				]);
		if (scenario === 'judged')
			for (const line of [1, 3, 5])
				expectedRows.push([name, marker, excludedMember, line, 'not_applicable', 'judged']);
	}
	equal(
		rows.map((row) => [
			row['name'],
			row['marker'],
			row['file'],
			row['first_line'],
			row['outcome'],
			row['basis'],
		]),
		expectedRows,
		'Ordered results differ',
	);
	for (const row of rows.filter((result) => result['outcome'] === 'not_applicable'))
		equal(
			row['reason'],
			{ kind: 'filter', filter: 'category', answer: 'blocked', probability: 0.9 },
			'Exclusion reason lost',
		);
	const skips = arrayValue(report['filter_skips']).map(objectValue);
	const skipped = scenario === 'unsure' || scenario === 'failed' || scenario === 'folderless';
	equal(
		skips.map((skip) => textValue(skip['rule'])).toSorted(),
		skipped ? ['asking_gate', 'mechanical_gate'] : [],
		'Skip identities/count differ',
	);
	for (const skip of skips) {
		equal(
			skip['file'],
			scenario === 'folderless' ? folderlessMember : excludedMember,
			'Skip file differs',
		);
		equal(skip['folder'], scenario === 'folderless' ? null : excludedFolder, 'Skip folder differs');
		equal(skip['filter'], 'category', 'Skip filter differs');
		equal(
			skip['why'],
			scenario === 'unsure' ? 'unsure' : scenario === 'failed' ? 'incomplete' : 'no_folder',
			'Skip reason differs',
		);
		equal(
			skip['probabilities'],
			scenario === 'unsure' ? { eligible: 0.3, blocked: 0.7 } : null,
			'Skip probabilities differ',
		);
	}
	const incomplete = arrayValue(report['incomplete']).map(objectValue);
	const parts = incomplete.map((part) => {
		const reason = objectValue(part['reason']);
		return [part['rule'], part['file'], reason['kind'], reason['filter']];
	});
	const expectedParts =
		scenario === 'failed'
			? [[null, excludedFolder, 'filter', 'category']]
			: scenario === 'folderless'
				? [
						['mechanical_gate', folderlessMember, 'no_filter_folder', 'category'],
						['asking_gate', folderlessMember, 'no_filter_folder', 'category'],
					]
				: [];
	equal(
		parts.map((part) => JSON.stringify(part)).toSorted(),
		expectedParts.map((part) => JSON.stringify(part)).toSorted(),
		'Incomplete explanation identities differ',
	);
}

async function observeFilterCase(options: {
	observer: RulebookProcess;
	root: string;
	scenario: FilterScenario;
}): Promise<void> {
	const { observer, root, scenario } = options;
	configureEndpoint(root, observer.endpoint);
	const checked = await observer.run({
		root,
		name: `${scenario}-check`,
		arguments: ['lint', '--config', configName, '--check'],
		expectedExit: 0,
	});
	equal(checked.captures.length, 0, 'Keyless check dispatched');
	observer.respond = ({ body }) =>
		Promise.resolve(
			scenario === 'failed' &&
				objectValue(objectValue(body['state'])['where'])['folder'] === excludedFolder
				? { status: 400, body: { error: 'controlled filter refusal' } }
				: { status: 200, body: filterReply(body, scenario) },
		);
	const commandArguments =
		scenario === 'folderless'
			? lintArguments(folderlessMember)
			: scenario === 'member'
				? lintArguments(includedMembers[0] ?? '')
				: lintArguments('--all');
	const result = await observer.report({
		root,
		name: scenario,
		arguments: commandArguments,
		credential: true,
		expectedExit:
			scenario === 'unsure' ? 5 : scenario === 'failed' || scenario === 'folderless' ? 3 : 0,
	});
	requireFilterRequests(result.captures, scenario);
	requireFilterReport(result.report, scenario);
	equal(
		objectValue(result.report['usage'])['model_requests'],
		result.captures.length,
		'Filter usage disappeared or doubled',
	);
}

async function observeUngated(observer: RulebookProcess, root: string): Promise<void> {
	configureEndpoint(root, observer.endpoint);
	const config = readObject(path.join(root, configName));
	const filters = config['filters'];
	requireCondition(filters, 'Fixture filter missing');
	delete config['filters'];
	writeJson(path.join(root, configName), config);
	const rules = readObject(path.join(root, rulesName));
	for (const rule of Object.values(rules)) delete objectValue(rule)['only_when'];
	writeJson(path.join(root, rulesName), rules);
	observer.respond = undefined;
	observer.reply = (body) => responseBody(body, 0.95);
	const baseline = await observer.run({
		root,
		name: 'ungated-absent',
		arguments: lintArguments('--all'),
		credential: true,
		expectedExit: 0,
	});
	configure(root, (value) => {
		value['filters'] = filters;
	});
	const declared = await observer.run({
		root,
		name: 'ungated-declared',
		arguments: lintArguments('--all'),
		credential: true,
		expectedExit: 0,
	});
	equal(
		declared.stdout.toString('utf8'),
		baseline.stdout.toString('utf8'),
		'Ungated JSON bytes changed',
	);
	equal(declared.captures.length, 6, 'Ungated rules were suppressed or filter was asked');
	equal(
		declared.captures.map(({ raw }) => raw.toString('utf8')).toSorted(),
		baseline.captures.map(({ raw }) => raw.toString('utf8')).toSorted(),
		'Ungated HTTP bodies changed',
	);
	const report = parseObject(declared.stdout.toString('utf8'));
	equal(arrayValue(report['results']).length, 12, 'Ungated positive control lost results');
	equal(report['filter_answers'], [], 'Ungated config asked filters');
	equal(report['filter_skips'], [], 'Ungated config fabricated skips');
}

export async function runFiltersJourney(options: JourneyArguments): Promise<void> {
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
	const scenarios: FilterScenario[] = ['judged', 'unsure', 'failed', 'folderless', 'member'];
	for (const scenario of [...scenarios, 'ungated']) {
		const root = prepareFilterFixture(path.join(evidence, scenario));
		if (scenario === 'folderless')
			filesystem.writeFileSync(
				path.join(root, folderlessMember),
				'Approved unowned instruction.\n',
			);
	}
	if (options.prepare) {
		writeJson(path.join(evidence, 'prepared.json'), {
			product_observed: false,
			oracle: filtersOracle,
			program_design: filtersProgramDesign,
			scenarios: [...scenarios, 'ungated'],
		});
		process.stdout.write('Prepared filters fixtures; product unobserved\n');
		return;
	}
	requireCondition(target, 'Filters observation requires the declared E5 candidate');
	requireDebugSignature(target.binary);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		await runSequence(scenarios, async (scenario) => {
			await observeFilterCase({ observer, root: path.join(evidence, scenario), scenario });
		});
		await observeUngated(observer, path.join(evidence, 'ungated'));
	} finally {
		await observer.close();
	}
	writeJson(path.join(evidence, 'pass.json'), {
		product_observed: true,
		commit: target.commit,
		binary_sha256: target.digest,
		oracle: filtersOracle,
		program_design: filtersProgramDesign,
		scenarios: [...scenarios, 'ungated'],
		required_native_joins: [
			'pre-walk gate',
			'same-process freshness',
			'change dependencies before selection and picking',
			'R6 admission matrix',
			'R9 cut and encoded request cap',
			'shared budget/cancellation',
			'activation and acquisition',
			'human/SARIF and schemas',
		],
		unverified: ['hosted classification accuracy', 'release publication'],
	});
	process.stdout.write(
		'Filters CLI journey passed; native joins and hosted accuracy remain separate\n',
	);
}
