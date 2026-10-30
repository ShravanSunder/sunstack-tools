/** Selection packing, shared views, target omission and contained provider/budget failures. */
import path from 'node:path';

import {
	codePoints,
	configName,
	configure,
	configureEndpoint,
	equal,
	objectValue,
	requireCondition,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import { type RulebookProcess } from './rulebook-process.ts';
import {
	requireCandidateKeys,
	requireSameShared,
	selectionEntries,
	selectionPieces,
	selectionState,
} from './selection-checks.ts';
import {
	askingStepRule,
	changeSelectionKinds,
	copySelectionFixture,
	keepKinds,
	researchFile,
	presentationFile,
	writeChecked,
} from './selection-fixture.ts';
import { scriptedSelectionReply, selectionArguments } from './selection-reply.ts';
function limits(root: string, values: JsonObject): void {
	configure(root, (config) => {
		config['limits'] = { ...objectValue(config['limits']), ...values };
	});
}
function customSections(root: string, bodies: readonly string[]): void {
	writeChecked(
		root,
		'skills/custom/SKILL.md',
		'# Candidates\n\n' +
			bodies.map((body, index) => `## ${index + 1}. Candidate\n\n1. ${body}\n\n`).join(''),
	);
}
export async function observeSelectionLimits(options: {
	observer: RulebookProcess;
	evidence: string;
	sourceRoot: string;
}): Promise<void> {
	const { observer, evidence, sourceRoot } = options;
	const base = copySelectionFixture({
		destination: path.join(evidence, 'packing'),
		sourceRoot,
		files: ['skills/custom/SKILL.md'],
	});
	keepKinds(base, ['workflow_section']);
	customSections(
		base,
		Array.from({ length: 5 }, () => 'é尾"\\ and escaped lines. '.repeat(130)),
	);
	changeSelectionKinds(base, (kinds) => {
		objectValue(kinds['workflow_section'])['max_chars'] = 20000;
	});
	configureEndpoint(base, observer.endpoint);
	observer.reply = scriptedSelectionReply({
		probability: (_file, key) => (key === 'c2' ? 0.12 : 0.93),
	});
	const generous = await observer.report({
		root: base,
		name: 'packing-measure',
		arguments: ['lint', '--config', configName, '--evidence', '--all'],
		expectedExit: 0,
		credential: true,
	});
	equal(generous.captures.length, 1, 'Generous body must fit five candidates');
	const captured = generous.captures[0];
	requireCondition(captured, 'Missing measured request');
	const original = captured.body;
	const state = selectionState(original);
	function measured(keys: readonly string[]): number {
		return codePoints(
			JSON.stringify({
				...original,
				questions: Object.fromEntries(
					keys.map((key) => [key, objectValue(original['questions'])[key]]),
				),
				state: {
					where: { file: state.file },
					candidates: Object.fromEntries(keys.map((key) => [key, state.candidates[key]])),
					shared: state.shared,
				},
			}),
		);
	}
	const cap = measured(['c1', 'c2', 'c3']);
	requireCondition(
		measured(['c1', 'c2', 'c3', 'c4']) > cap && measured(['c4', 'c5']) <= cap,
		'Packing fixture cannot separate3/2',
	);
	limits(base, { request_max_chars: cap });
	const split = await observer.report({
		root: base,
		name: 'packing-three-two',
		arguments: ['lint', '--config', configName, '--evidence', '--all'],
		expectedExit: 0,
		credential: true,
	});
	requireCandidateKeys({
		bodies: split.captures.map((capture) => capture.body),
		groups: [
			['c1', 'c2', 'c3'],
			['c4', 'c5'],
		],
	});
	requireSameShared(split.captures.map((capture) => capture.body));
	customSections(base, [
		'small one',
		'small two',
		'small three',
		'é尾"\\'.repeat(8000),
		'small five',
	]);
	limits(base, { request_max_chars: cap });
	const large = await observer.report({
		root: base,
		name: 'packing-one-too-large',
		arguments: ['lint', '--config', configName, '--evidence', '--all'],
		expectedExit: 3,
		credential: true,
	});
	const incomplete = selectionEntries(large.report).filter(
		(entry) => entry.outcome.kind === 'incomplete',
	);
	requireCondition(
		incomplete.length === 1 &&
			incomplete[0]?.outcome.kind === 'incomplete' &&
			incomplete[0].outcome.reason['kind'] === 'request_too_large',
		'Large isolated candidate not refused precisely',
	);
	requireCondition(
		large.captures.every((capture) => !('c4' in objectValue(capture.body['questions']))),
		'Over-limit c4 sent',
	);
	const unequal = copySelectionFixture({
		destination: path.join(evidence, 'shared-unequal'),
		sourceRoot,
		files: ['skills/custom/SKILL.md'],
	});
	keepKinds(unequal, ['workflow_section']);
	customSections(unequal, ['Short body.', 'Oversize middle token '.repeat(800)]);
	changeSelectionKinds(unequal, (kinds) => {
		objectValue(kinds['workflow_section'])['max_chars'] = 1000;
	});
	configureEndpoint(unequal, observer.endpoint);
	const bounded = await observer.report({
		root: unequal,
		name: 'shared-view-unequal-candidates',
		arguments: ['lint', '--config', configName, '--evidence', '--all'],
		expectedExit: 0,
		credential: true,
	});
	for (const capture of bounded.captures) {
		const selected = selectionState(capture.body);
		for (const candidate of Object.values(selected.candidates))
			requireCondition(modelCharacters(candidate) <= 1000, 'Candidate exceeds max_chars');
		requireCondition(
			modelCharacters(selected.shared) <= 1000,
			'Shared exceeds independent max_chars',
		);
	}
	requireSameShared(bounded.captures.map((capture) => capture.body));
	const targets = copySelectionFixture({
		destination: path.join(evidence, 'target-omission'),
		sourceRoot,
		files: ['skills/custom/SKILL.md', 'skills/target/SKILL.md'],
		allKinds: true,
	});
	keepKinds(targets, ['selected_targets', 'z_skill', 'plain_heading']);
	configure(targets, (config) => {
		config['rulebook'] = [{ markdown: 'rulebook.md', rules: 'target-rules.json' }];
	});
	writeJson(path.join(targets, 'target-rules.json'), {
		K001: {
			name: 'selected_body',
			evidence: 'selected_targets',
			decision: {
				start: 'body',
				steps: {
					body: {
						mechanical_check: { tool: 'contains', field: 'body', text: '1.' },
						yes: 'pass',
						['no']: 'pass',
					},
				},
			},
		},
	});
	writeChecked(
		targets,
		'skills/custom/SKILL.md',
		'# Selection\n\n## Workflow\n\n1. load `../target/SKILL.md` and `target`.\n',
	);
	writeChecked(
		targets,
		'skills/target/SKILL.md',
		'---\nname: target\ndescription: TARGET_FIELDS_SENTINEL\n---\n\n# Target\n\nOPENING_SENTINEL\n',
	);
	configureEndpoint(targets, observer.endpoint);
	const omitted = await observer.report({
		root: targets,
		name: 'selection-omits-target-fields',
		arguments: ['lint', '--config', configName, '--evidence', '--all'],
		expectedExit: 0,
		credential: true,
	});
	requireCondition(omitted.captures.length > 0, 'Target fixture asked nothing');
	for (const capture of omitted.captures) {
		const raw = capture.raw.toString('utf8');
		requireCondition(
			!raw.includes('TARGET_FIELDS_SENTINEL') &&
				!raw.includes('OPENING_SENTINEL') &&
				!raw.includes('"path_targets"') &&
				!raw.includes('"skill_targets"'),
			'Selection request carries target fields/opening',
		);
	}
	const selected = selectionPieces(omitted.report, 'selected_targets');
	requireCondition(selected.length === 1, 'Target candidate did not become a piece');
	const fields = objectValue(selected[0]?.['fields']);
	requireCondition(
		'path_targets' in fields && 'skill_targets' in fields,
		'Selected piece lost target fields',
	);
	const pathTargets = objectValue(fields['path_targets']);
	const skillTargets = objectValue(fields['skill_targets']);
	requireCondition(
		JSON.stringify(pathTargets).includes('OPENING_SENTINEL') &&
			JSON.stringify(pathTargets).includes('"exists":true'),
		'Selected path target did not retain its real opening',
	);
	requireCondition(
		JSON.stringify(skillTargets).includes('TARGET_FIELDS_SENTINEL') &&
			JSON.stringify(skillTargets).includes('"exists":true'),
		'Selected skill target did not compose its final fields',
	);
	const failed = copySelectionFixture({
		destination: path.join(evidence, 'provider-failure'),
		sourceRoot,
		files: [researchFile, presentationFile],
	});
	askingStepRule(failed);
	configureEndpoint(failed, observer.endpoint);
	observer.respond = async (request) => {
		const requestState = objectValue(request.body['state']);
		return 'candidates' in requestState && selectionState(request.body).file === presentationFile
			? { status: 500, body: { error: 'controlled provider failure' } }
			: { status: 200, body: observer.reply(request.body) };
	};
	let result;
	try {
		result = await observer.report({
			root: failed,
			name: 'selection-provider-contained',
			arguments: selectionArguments('--all'),
			expectedExit: 3,
			credential: true,
		});
	} finally {
		observer.respond = undefined;
	}
	const rows = selectionEntries(result.report);
	requireCondition(
		rows.some(
			(entry) =>
				entry.where.file === presentationFile &&
				entry.outcome.kind === 'incomplete' &&
				entry.outcome.reason['kind'] === 'provider_failure',
		),
		'Provider failure not contained/reported',
	);
	requireCondition(
		rows
			.filter((entry) => entry.where.file === researchFile)
			.every((entry) => entry.outcome.kind === 'selected'),
		'Peer file selection was discarded',
	);
	requireCondition(
		result.captures.some((capture) => !('candidates' in objectValue(capture.body['state']))),
		'Rules did not run on surviving pieces',
	);
	const budget = copySelectionFixture({
		destination: path.join(evidence, 'shared-budget'),
		sourceRoot,
		files: [researchFile, presentationFile],
	});
	askingStepRule(budget);
	configureEndpoint(budget, observer.endpoint);
	limits(budget, { max_model_requests: 3, max_concurrent_requests: 1 });
	const limited = await observer.report({
		root: budget,
		name: 'selection-and-rules-share-budget',
		arguments: selectionArguments('--all'),
		expectedExit: 3,
		credential: true,
	});
	equal(limited.captures.length, 3, 'Selection and rules exceeded/shared wrong request budget');
	equal(
		objectValue(limited.report['usage'])['model_requests'],
		3,
		'Usage missing selection requests',
	);
	requireCondition(
		JSON.stringify(limited.report).includes('request_limit_reached'),
		'Shared budget did not refuse remaining work',
	);
	const tokens = copySelectionFixture({
		destination: path.join(evidence, 'token-limit'),
		sourceRoot,
		files: ['skills/custom/SKILL.md'],
	});
	keepKinds(tokens, ['workflow_section']);
	customSections(tokens, ['Small.', 'over token budget '.repeat(1000), 'Small peer.']);
	configureEndpoint(tokens, observer.endpoint);
	limits(tokens, { max_input_tokens_per_request: 1000 });
	const tokenLimited = await observer.report({
		root: tokens,
		name: 'selection-input-token-limit',
		arguments: ['lint', '--config', configName, '--evidence', '--all'],
		expectedExit: 3,
		credential: true,
	});
	const refused = selectionEntries(tokenLimited.report).filter(
		(entry) => entry.outcome.kind === 'incomplete',
	);
	requireCondition(
		refused.length === 1 &&
			JSON.stringify(refused).includes('request_too_large') &&
			JSON.stringify(refused).includes('input_tokens'),
		'Input-token guard missing precise refusal',
	);
	requireCondition(
		tokenLimited.captures.every((capture) => !('c2' in objectValue(capture.body['questions']))) &&
			tokenLimited.captures.length > 0,
		'Token-over-limit candidate sent or peer suppressed',
	);
}
function modelCharacters(value: unknown): number {
	if (typeof value === 'string') return codePoints(value);
	if (Array.isArray(value))
		return value.reduce((total: number, item: unknown) => total + modelCharacters(item), 0);
	if (value && typeof value === 'object')
		return Object.values(value).reduce(
			(total: number, item: unknown) => total + modelCharacters(item),
			0,
		);
	return 0;
}
