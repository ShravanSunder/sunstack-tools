/** O6 fixture Git revisions, source hashes and authored case expectations. */
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import { excludedMember, openings, includedFolder, includedMembers } from './rulebook-filters.ts';
import {
	baselineGit,
	configName,
	configure,
	gitCommand,
	objectValue,
	readObject,
	requireCondition,
	rulebookName,
	rulesName,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import { prepareReportFixture, gapReportFile, skippedReportFolder } from './rulebook-report.ts';

export const casesOracle =
	'aa2f850:docs/specs/2026-11-06-decision-trees-everywhere/verification-design.html#o6';
export const casesConfigName = 'configuration/skill-rules.config.json';
export const historicReferenceFile = `${includedFolder}/references/history.md`;
export const historicalOpening = 'HISTORICAL_EXTERNAL_OPENING_SENTINEL.\n';
export const currentOpening = 'CURRENT_EXTERNAL_OPENING_SENTINEL.\n';
export const historicalDescription = 'General instructions for an included folder.';
const ambiguousFile = `${includedFolder}/notes/ambiguous.md`;
const unmatchedFile = `${includedFolder}/notes/unmatched.md`;
const mustPassFile = `${includedFolder}/notes/must-pass.md`;

function contentHash(text: string): string {
	return createHash('sha256').update(text).digest('hex');
}
function ruleCase(options: {
	root: string;
	id: string;
	file: string;
	revision: string;
	expected: string;
	lines?: number[];
}): JsonObject {
	const lines = options.lines ?? [1, 1];
	const content = filesystem.readFileSync(path.join(options.root, options.file), 'utf8');
	const selected = content
		.split(/(?<=\n)/)
		.slice((lines[0] ?? 1) - 1, lines[1] ?? 1)
		.join('');
	return {
		id: options.id,
		kind: 'rule',
		rule: 'asking_gate',
		file: options.file,
		revision: options.revision,
		lines,
		text_sha256: contentHash(selected),
		expected: options.expected,
		set: 'development',
		label_from: 'blind-agents',
	};
}

function fileAt(index: number): string {
	const file = includedMembers[index];
	requireCondition(file, 'Missing case member');
	return file;
}

/** Whole-repository revisions distinguish historical filter and external load context from today's files. */
export function prepareCasesFixture(destination: string): JsonObject {
	prepareReportFixture(destination);
	filesystem.mkdirSync(path.join(destination, includedFolder, 'references'), { recursive: true });
	filesystem.mkdirSync(path.join(destination, 'external'), { recursive: true });
	filesystem.writeFileSync(path.join(destination, 'external/context.md'), historicalOpening);
	filesystem.writeFileSync(
		path.join(destination, historicReferenceFile),
		'Approved HISTORICAL_REFERENCE_BODY_SENTINEL. Read `../../../../../external/context.md`.\n',
	);
	filesystem.writeFileSync(
		path.join(destination, ambiguousFile),
		'Approved first AMBIGUOUS_BODY_SENTINEL.\n\nApproved second AMBIGUOUS_BODY_SENTINEL.\n',
	);
	filesystem.writeFileSync(path.join(destination, unmatchedFile), '# UNMATCHED_HEADING_SENTINEL\n');
	filesystem.writeFileSync(
		path.join(destination, mustPassFile),
		'Approved MUST_PASS_BODY_SENTINEL.\n',
	);
	configure(destination, (config) => {
		config['check'] = { files: ['plugins/**/notes/*.md', 'plugins/**/references/*.md'] };
		objectValue(config['decision_model'])['endpoint'] = 'http://127.0.0.1:9/systemone';
		const kinds = objectValue(objectValue(config['evidence'])['kinds']);
		objectValue(objectValue(kinds['subject'])['fields'])['targets'] = {
			from: 'target',
			regex: '`([^`]+)`',
			resolve: 'path_or_skill',
		};
	});
	const rules = readObject(path.join(destination, rulesName));
	const asking = objectValue(rules['K002']);
	asking['questions'] = {
		verdict: {
			type: 'noul',
			instructions: {
				check: 'body',
				context: ['targets', 'reference.examples'],
				task: 'Is this instruction approved in its loaded context?',
			},
		},
	};
	writeJson(path.join(destination, rulesName), rules);
	const nestedConfig = readObject(path.join(destination, configName));
	nestedConfig['rulebook'] = [{ markdown: `../${rulebookName}`, rules: `../${rulesName}` }];
	objectValue(nestedConfig['evidence'])['ast_grep'] = '../evidence/sgconfig.yml';
	filesystem.mkdirSync(path.join(destination, 'configuration'));
	writeJson(path.join(destination, casesConfigName), nestedConfig);
	filesystem.unlinkSync(path.join(destination, configName));
	baselineGit(destination);
	const historical = gitCommand(destination, ['rev-parse', 'HEAD']);

	const cases = [
		ruleCase({
			root: destination,
			id: 'pass',
			file: fileAt(0),
			revision: historical,
			expected: 'pass',
		}),
		ruleCase({
			root: destination,
			id: 'must-pass',
			file: mustPassFile,
			revision: historical,
			expected: 'pass',
		}),
		ruleCase({
			root: destination,
			id: 'must-warn',
			file: fileAt(1),
			revision: historical,
			expected: 'warn',
		}),
		ruleCase({
			root: destination,
			id: 'unsure',
			file: fileAt(2),
			revision: historical,
			expected: 'pass',
		}),
		ruleCase({
			root: destination,
			id: 'incomplete',
			file: gapReportFile,
			revision: historical,
			expected: 'pass',
		}),
		ruleCase({
			root: destination,
			id: 'stale',
			file: fileAt(0),
			revision: historical,
			expected: 'pass',
		}),
		ruleCase({
			root: destination,
			id: 'unmatched',
			file: unmatchedFile,
			revision: historical,
			expected: 'pass',
		}),
		ruleCase({
			root: destination,
			id: 'ambiguous',
			file: ambiguousFile,
			revision: historical,
			expected: 'pass',
			lines: [1, 3],
		}),
		ruleCase({
			root: destination,
			id: 'excluded-must-pass',
			file: excludedMember,
			revision: historical,
			expected: 'pass',
		}),
		ruleCase({
			root: destination,
			id: 'historical-reference',
			file: historicReferenceFile,
			revision: historical,
			expected: 'pass',
		}),
	];
	const stale = cases.find((item) => item['id'] === 'stale');
	requireCondition(stale, 'Missing stale case');
	stale['text_sha256'] = '0'.repeat(64);
	// Hash authored unprojected declared values; normal lint report hashes are checked against these literals.
	const filterCases = [
		{
			id: 'filter-right',
			kind: 'filter',
			filter: 'category',
			folder: includedFolder,
			reads_sha256: contentHash(
				JSON.stringify({
					description: historicalDescription,
					opening: openings.get(includedFolder),
				}),
			),
			revision: historical,
			expected_label: 'eligible',
			set: 'development',
			label_from: 'blind-agents',
		},
		{
			id: 'filter-wrong',
			kind: 'filter',
			filter: 'category',
			folder: includedFolder,
			reads_sha256: contentHash(
				JSON.stringify({
					description: historicalDescription,
					opening: openings.get(includedFolder),
				}),
			),
			revision: historical,
			expected_label: 'blocked',
			set: 'development',
			label_from: 'blind-agents',
		},
		{
			id: 'filter-unsure',
			kind: 'filter',
			filter: 'category',
			folder: skippedReportFolder,
			reads_sha256: contentHash(
				JSON.stringify({ description: 'Uncertain report guidance.', opening: 'Unsure opening.' }),
			),
			revision: historical,
			expected_label: 'eligible',
			set: 'development',
			label_from: 'blind-agents',
		},
	];

	const currentRules = readObject(path.join(destination, rulesName));
	const currentRule = objectValue(currentRules['K002']);
	objectValue(currentRule['reference'])['provenance'] = 'CURRENT_RULEBOOK_REFERENCE_SENTINEL';
	objectValue(objectValue(objectValue(currentRule['questions'])['verdict'])['instructions'])[
		'context'
	] = ['targets', 'reference.examples', 'reference.provenance'];
	writeJson(path.join(destination, rulesName), currentRules);
	filesystem.writeFileSync(path.join(destination, 'external/context.md'), currentOpening);
	const overview = path.join(destination, includedFolder, 'overview.md');
	filesystem.writeFileSync(
		overview,
		filesystem
			.readFileSync(overview, 'utf8')
			.replace(historicalDescription, 'CURRENT_FILTER_DESCRIPTION_SENTINEL.'),
	);
	filesystem.writeFileSync(
		path.join(destination, historicReferenceFile),
		'Approved CURRENT_REFERENCE_BODY_SENTINEL. Read `../../../../../external/context.md`.\n',
	);
	gitCommand(destination, ['add', '.']);
	gitCommand(destination, ['commit', '-qm', 'Current tree differs from historical case context']);
	const current = gitCommand(destination, ['rev-parse', 'HEAD']);
	cases.push(
		ruleCase({
			root: destination,
			id: 'current-pass',
			file: fileAt(0),
			revision: current,
			expected: 'pass',
		}),
		ruleCase({
			root: destination,
			id: 'current-reference',
			file: historicReferenceFile,
			revision: current,
			expected: 'pass',
		}),
	);
	return {
		product_observed: false,
		oracle: casesOracle,
		historical,
		current,
		config: casesConfigName,
		rule_cases: cases,
		filter_cases: filterCases,
		expected_dispositions: {
			pass: 'right',
			'must-pass': 'right',
			'must-warn': 'right',
			unsure: 'unsure',
			incomplete: 'incomplete',
			stale: 'stale',
			unmatched: 'no_matching_piece',
			ambiguous: 'more_than_one_matching_piece',
			'excluded-must-pass': 'filter_excluded',
			'historical-reference': 'right',
			'current-pass': 'right',
			'current-reference': 'right',
			'filter-right': 'right',
			'filter-wrong': 'wrong_label',
			'filter-unsure': 'unsure',
		},
		historical_context: {
			filter_description: historicalDescription,
			external_opening: historicalOpening,
			reference_body: 'HISTORICAL_REFERENCE_BODY_SENTINEL',
		},
		bars: [
			{
				for: 'asking_gate',
				step: 'approved',
				bound: 'yes_at_or_above',
				value: 0.7,
				set_from: ['must-warn', 'must-pass'],
			},
			{
				for: 'asking_gate',
				step: 'approved',
				bound: 'no_at_or_below',
				value: 0.3,
				set_from: ['must-warn', 'must-pass'],
			},
			{
				for: 'category',
				step: 'scope',
				bound: 'min_probability',
				value: 0.7,
				set_from: ['filter-right'],
			},
			{
				for: 'category',
				step: 'style',
				bound: 'min_probability',
				value: 0.7,
				set_from: ['filter-right'],
			},
		],
	};
}
