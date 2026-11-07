/** O4: tree filters through real lint; loopback replaces only Jev answers. */
import filesystem from 'node:fs';
import path from 'node:path';

import { requireDebugSignature } from './rulebook-amendment.ts';
import {
	arrayValue,
	configName,
	configureEndpoint,
	equal,
	journeyArguments,
	lintArguments,
	numberValue,
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

export const filtersOracle =
	'aa2f850:docs/specs/2026-11-06-decision-trees-everywhere/verification-design.html#o4';
export const includedFolder = 'plugins/one/skills/review';
export const excludedFolder = 'plugins/two/skills/review';
export const includedMembers = ['one.md', 'two.md', 'three.md'].map(
	(name) => `${includedFolder}/notes/${name}`,
);
export const excludedMember = `${excludedFolder}/notes/one.md`;
export const folderlessMember = 'elsewhere/unowned.md';
export const filterReference: JsonObject = {
	kinds: { reusable: 'General instructions.', product: 'Specific instructions.' },
	styles: 'Approved guidance is eligible.',
};
export const filterQuestions: JsonObject = {
	scope: {
		type: 'choice',
		instructions: {
			check: 'description',
			against: 'reference.kinds',
			task: 'Classify the primary work.',
		},
		criteria: {
			reusable: 'Reusable guidance.',
			product: 'A product procedure.',
			other: 'Other work.',
		},
	},
	style: {
		type: 'choice',
		instructions: { check: 'opening', against: 'reference.styles', task: 'Classify the guidance.' },
		criteria: {
			approved: 'Approved guidance.',
			blocked: 'Blocked guidance.',
			other: 'Other guidance.',
		},
	},
};
export const filterDecision: JsonObject = {
	start: 'scope',
	steps: {
		scope: {
			ask: 'scope',
			min_probability: 0.7,
			routes: { reusable: 'style', product: { label: 'blocked' }, other: { label: 'blocked' } },
			uncertain: 'unsure',
		},
		style: {
			ask: 'style',
			min_probability: 0.7,
			routes: {
				approved: { label: 'eligible' },
				blocked: { label: 'blocked' },
				other: { label: 'blocked' },
			},
			uncertain: 'unsure',
		},
	},
};
export const descriptions = new Map([
	[includedFolder, 'General instructions for an included folder.'],
	[excludedFolder, 'Specific instructions for an excluded folder.'],
]);
export const openings = new Map([
	[includedFolder, 'Approved reusable guidance.'],
	[excludedFolder, 'Product operating procedure.'],
]);
export type FilterScenario =
	| 'judged'
	| 'root-unsure'
	| 'unsure'
	| 'failed'
	| 'folderless'
	| 'member';

/** Expected fixture values are authored independently of product output. */
export function prepareFilterFixture(destination: string): string {
	requireCondition(!filesystem.existsSync(destination), 'Filters fixture already exists');
	for (const directory of ['rulebook', 'evidence/rules', 'elsewhere'])
		filesystem.mkdirSync(path.join(destination, directory), { recursive: true });
	writeJson(path.join(destination, 'evidence/sgconfig.yml'), { ruleDirs: ['rules'] });
	filesystem.writeFileSync(path.join(destination, 'evidence/rules/.gitkeep'), '');
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
						['no']: { finding: 'Missing approval; read the instruction and decide.' },
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
					instructions: { check: 'body', against: 'where', task: 'Is this instruction approved?' },
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
						uncertain: 'uncertain',
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
					fields: {
						description: { from: 'yaml', key: 'description' },
						opening: { from: 'yaml', key: 'opening' },
					},
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
				reads: { file: 'overview.md', kind: 'description', fields: ['description', 'opening'] },
				labels: ['eligible', 'blocked'],
				questions: filterQuestions,
				decision: filterDecision,
				reference: filterReference,
			},
		},
		decision_model: { model: 'typesafe/requested-filters' },
	});
	for (const [folder, description] of descriptions) {
		filesystem.mkdirSync(path.join(destination, folder, 'notes'), { recursive: true });
		filesystem.writeFileSync(
			path.join(destination, folder, 'overview.md'),
			`---\ndescription: ${description}\nopening: ${openings.get(folder)}\n---\n\nCONTEXT_ONLY_SENTINEL.\n`,
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
export function filterFolder(body: JsonObject): string | undefined {
	if (!('scope' in objectValue(body['questions']))) return undefined;
	const description = objectValue(body['state'])['description'];
	const entry = [...descriptions].find(([, expected]) => expected === description);
	requireCondition(entry, 'Filter asked about unexpected description');
	return entry[0];
}
function choiceAnswer(selected: string, labels: readonly string[], unsure: boolean): JsonObject {
	return {
		type: 'choice',
		choice: selected,
		confidence: 0.99,
		probabilities: Object.fromEntries(
			labels.map((label) => [
				label,
				label === selected ? (unsure ? 0.5 : 0.9) : unsure ? 0.25 : 0.05,
			]),
		),
	};
}
export function filterReply(body: JsonObject, scenario: FilterScenario): JsonObject {
	const reply = responseBody(body, 0.95);
	const folder = filterFolder(body);
	if (!folder) return reply;
	const excluded = folder === excludedFolder;
	const answers = objectValue(reply['answers']);
	answers['scope'] = choiceAnswer(
		excluded && scenario === 'judged' ? 'product' : 'reusable',
		['reusable', 'product', 'other'],
		excluded && scenario === 'root-unsure',
	);
	answers['style'] = choiceAnswer(
		'approved',
		['approved', 'blocked', 'other'],
		excluded && scenario === 'unsure',
	);
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
		const folder = filterFolder(body);
		if (folder) {
			folders.push(folder);
			equal(body['questions'], filterQuestions, 'Filter questions changed or split');
			equal(
				state,
				{
					description: descriptions.get(folder),
					opening: openings.get(folder),
					reference: filterReference,
					'unavailable-fields': {},
				},
				'Filter state must carry only declared fields and reference',
			);
		} else {
			ruleFiles.push(textValue(objectValue(state['where'])['file']));
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
		'Excluded/skipped members asked or eligible members omitted',
	);
}
/** Closed tree outcome oracle; the candidate adapter supplies these named public values. */
export function requireFilterWalk(options: {
	state: unknown;
	label: unknown;
	path: unknown;
	step: unknown;
	answers: JsonObject;
	scenario: FilterScenario;
	folder: string;
}): void {
	const excluded = options.folder === excludedFolder;
	const incomplete = excluded && options.scenario === 'failed';
	const unsure = excluded && ['root-unsure', 'unsure'].includes(options.scenario);
	const expectedPath = incomplete
		? []
		: excluded && ['judged', 'root-unsure'].includes(options.scenario)
			? ['scope']
			: ['scope', 'style'];
	equal(
		options.state,
		incomplete ? 'incomplete' : unsure ? 'unsure' : 'judged',
		'Wrong closed filter state',
	);
	equal(options.path, expectedPath, 'Filter route lost or fabricated');
	equal(
		options.label,
		incomplete || unsure ? undefined : excluded ? 'blocked' : 'eligible',
		'Wrong or fabricated terminal label',
	);
	if (unsure)
		equal(
			options.step,
			options.scenario === 'root-unsure' ? 'scope' : 'style',
			'Unsure step differs',
		);
	const request: JsonObject = {
		questions: filterQuestions,
		state: { description: descriptions.get(options.folder) ?? '' },
	};
	const expected = objectValue(filterReply(request, options.scenario)['answers']);
	for (const step of expectedPath) {
		const raw = objectValue(expected[step]);
		equal(
			options.answers[step],
			{
				type: 'choice',
				selected_label: raw['choice'],
				confidence: raw['confidence'],
				probabilities: raw['probabilities'],
			},
			'Walked answer lost or changed',
		);
	}
}

export function requireFilterReport(report: JsonObject, scenario: FilterScenario): void {
	const skipped = ['root-unsure', 'unsure', 'failed', 'folderless'].includes(scenario);
	equal(
		objectValue(report['status'])['kind'],
		['root-unsure', 'unsure'].includes(scenario)
			? 'uncertain'
			: ['failed', 'folderless'].includes(scenario)
				? 'incomplete'
				: 'clean',
		'Wrong filter status',
	);
	equal(report['findings'], [], 'Filters fabricated warnings');
	equal(report['uncertain'], [], 'Filter unsure is not a rule result');
	const rows = arrayValue(report['results']).map(objectValue);
	const included =
		scenario === 'folderless'
			? []
			: scenario === 'member'
				? includedMembers.slice(0, 1)
				: includedMembers;
	for (const rule of ['mechanical_gate', 'asking_gate']) {
		const ruleRows = rows.filter((row) => row['name'] === rule);
		equal(
			ruleRows
				.filter((row) => row['outcome'] === 'pass')
				.map((row) => textValue(row['file']))
				.toSorted(),
			included.toSorted(),
			'Included pairs did not walk',
		);
		equal(
			ruleRows.filter((row) => row['file'] === excludedMember).map((row) => row['outcome']),
			scenario === 'judged' ? ['not_applicable', 'not_applicable', 'not_applicable'] : [],
			'Excluded/skipped pairs walked',
		);
	}
	equal(
		rows.length,
		included.length * 2 + (scenario === 'judged' ? 6 : 0),
		'Unexpected or missing result',
	);
	equal(
		arrayValue(report['filter_skips'])
			.map(objectValue)
			.map((skip) => textValue(skip['rule']))
			.toSorted(),
		skipped ? ['asking_gate', 'mechanical_gate'] : [],
		'Skips lost or duplicated',
	);
	const answers = arrayValue(report['filter_answers']).map(objectValue);
	equal(
		answers.map((answer) => textValue(answer['folder'])).toSorted(),
		(scenario === 'folderless'
			? []
			: scenario === 'member'
				? [includedFolder]
				: [includedFolder, excludedFolder]
		).toSorted(),
		'Filter result identities differ',
	);

	equal(report['counts'], { error: 0, warn: 0 }, 'Filter fabricated severity counts');
	equal(
		objectValue(report['exclusions'])['not_applicable'],
		scenario === 'judged' ? 6 : 0,
		'Skips became exclusions',
	);
	for (const answer of answers) {
		const folder = textValue(answer['folder']);
		equal(answer['filter'], 'category', 'Filter identity lost');
		equal(answer['requested_model'], 'typesafe/requested-filters', 'Requested model lost');
		equal(
			answer['resolved_model'],
			folder === excludedFolder && scenario === 'failed' ? null : 'typesafe/controlled-rulebook',
			'Responding model lost or invented',
		);
		equal(answer['input_cut'], false, 'Short context reported cut');
		requireFilterWalk({
			state: answer['state'],
			label: answer['label'],
			path: answer['path'],
			step: answer['step'],
			answers: objectValue(answer['answers']),
			scenario,
			folder,
		});
		requireCondition(
			!('probability' in answer) && !('probabilities' in answer),
			'Filter invented combined label confidence',
		);
		if (folder === excludedFolder && scenario === 'failed') {
			const reason = objectValue(answer['reason']);
			equal(reason['kind'], 'request', 'Failure reason lost');
			const failure = objectValue(objectValue(reason['reason'])['provider_failure']);
			equal(
				failure,
				{ failure_class: 'provider_rejected_request', http_status: 400 },
				'Provider failure changed',
			);
		}
	}
	for (const skip of arrayValue(report['filter_skips']).map(objectValue)) {
		equal(skip['filter'], 'category', 'Skip filter lost');
		equal(
			skip['file'],
			scenario === 'folderless' ? folderlessMember : excludedMember,
			'Skip file differs',
		);
		equal(skip['folder'], scenario === 'folderless' ? null : excludedFolder, 'Skip folder differs');
		equal(
			skip['why'],
			scenario === 'folderless' ? 'no_folder' : scenario === 'failed' ? 'incomplete' : 'unsure',
			'Skip reason differs',
		);
		const answer = answers.find((item) => item['folder'] === excludedFolder);
		equal(skip['path'], answer ? answer['path'] : [], 'Skip route lost');
		equal(skip['answers'], answer ? answer['answers'] : {}, 'Skip answers lost');
		const locations = arrayValue(skip['locations']).map(objectValue);
		for (const location of locations) {
			equal(location['file'], skip['file'], 'Skip location belongs to another file');
			equal(location['last_line'], location['first_line'], 'Skip location widened across pieces');
		}
		equal(
			locations
				.map((location) => numberValue(location['first_line']))
				.toSorted((leftLine, rightLine) => leftLine - rightLine),
			scenario === 'folderless' ? [1] : [1, 3, 5],
			'Skipped piece locations missing or duplicated',
		);
	}
	for (const row of rows.filter((item) => item['outcome'] === 'not_applicable'))
		equal(
			row['reason'],
			{ kind: 'filter', filter: 'category', label: 'blocked' },
			'Exclusion label/reason lost',
		);
	const incomplete = arrayValue(report['incomplete']).map(objectValue);
	equal(
		incomplete.length,
		scenario === 'failed' ? 1 : scenario === 'folderless' ? 2 : 0,
		'Incomplete explanations lost or invented',
	);
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
	const scenarios: FilterScenario[] = [
		'judged',
		'root-unsure',
		'unsure',
		'failed',
		'folderless',
		'member',
	];
	for (const scenario of scenarios) {
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
			scenarios,
		});
		process.stdout.write('Prepared tree filter fixtures; product unobserved\n');
		return;
	}
	requireCondition(target, 'Observation requires declared target');
	requireDebugSignature(target.binary);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		await runSequence(scenarios, async (scenario) => {
			const root = path.join(evidence, scenario);
			configureEndpoint(root, observer.endpoint);
			const check = await observer.run({
				root,
				name: `${scenario}-check`,
				arguments: ['lint', '--config', configName, '--check'],
				expectedExit: 0,
			});
			equal(check.captures.length, 0, 'Check dispatched');
			observer.respond = ({ body }) =>
				Promise.resolve(
					scenario === 'failed' && filterFolder(body) === excludedFolder
						? { status: 400, body: { error: 'controlled filter refusal' } }
						: { status: 200, body: filterReply(body, scenario) },
				);
			const result = await observer.report({
				root,
				name: scenario,
				arguments: lintArguments(
					scenario === 'folderless'
						? folderlessMember
						: scenario === 'member'
							? (includedMembers[0] ?? '')
							: '--all',
				),
				credential: true,
				expectedExit: ['root-unsure', 'unsure'].includes(scenario)
					? 5
					: ['failed', 'folderless'].includes(scenario)
						? 3
						: 0,
			});
			requireFilterRequests(result.captures, scenario);
			requireFilterReport(result.report, scenario);
			equal(
				objectValue(result.report['usage'])['model_requests'],
				result.captures.length,
				'Usage differs from real requests',
			);
		});
	} finally {
		await observer.close();
	}
	writeJson(path.join(evidence, 'pass.json'), {
		product_observed: true,
		commit: target.commit,
		binary_sha256: target.digest,
		oracle: filtersOracle,
		scenarios,
		unverified: [
			'zero mechanical walks requires native join',
			'hosted accuracy',
			'release publication',
		],
	});
	process.stdout.write('Tree filters CLI journey passed\n');
}
if (process.argv[1] === import.meta.filename) {
	const options = journeyArguments(
		'O4 tree filters, shared gates, routes and skips through real lint.',
	);
	if (options) await runFiltersJourney(options);
}
