/** Amended R30: union of final containers, inclusive equality, overlap once, and selection fallback. */
import filesystem from 'node:fs';
import path from 'node:path';

import {
	arrayValue,
	configName,
	configure,
	equal,
	jsonValueSchema,
	numberValue,
	objectValue,
	requireCondition,
	runSequence,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { type RulebookProcess } from '../rulebook/rulebook-process.ts';
import { selectionPieces } from './selection-checks.ts';
import { copySelectionFixture, writeChecked } from './selection-fixture.ts';
import { scriptedSelectionReply } from './selection-reply.ts';

const skillFile = 'skills/within-union/SKILL.md';
const referenceFile = 'references/within-standards.md';
const skillText =
	'# Union fixture\n\nOutside both containers.\n\n## Workflow\n\n1. Chosen step guidance.\n\n## Standards\n\nStandards-only guidance.\n\n1. Overlapping guidance.\n';
const referenceText = '# Reference\n\n## Standards\n\n1. Reference guidance.\n';

/** Compare counters with authored match facts, independently of the returned piece list. */
export function requireUnionCounts(options: {
	report: JsonObject;
	expected: readonly { file: string; matches: number; pieces: number; picked: number }[];
}): void {
	const counts = arrayValue(options.report['file_counts']).map(objectValue);
	equal(
		counts.map((row) => row['file']),
		options.expected.map((row) => row.file),
		'Union count file identities drifted',
	);
	for (const expected of options.expected) {
		const file = counts.find((row) => row['file'] === expected.file);
		requireCondition(file, 'Union count file missing');
		const sentence = objectValue(objectValue(file['kinds'])['sentence']);
		equal(
			[sentence['matches'], sentence['pieces'], sentence['picked']],
			[expected.matches, expected.pieces, expected.picked],
			'Union counters must match fixture facts for ' + expected.file,
		);
	}
}

function expectUnionCounts(report: JsonObject, chosen: boolean): void {
	requireUnionCounts({
		report,
		expected: [
			{ file: referenceFile, matches: 1, pieces: 1, picked: 1 },
			{ file: skillFile, matches: 4, pieces: chosen ? 3 : 2, picked: chosen ? 3 : 2 },
		],
	});
}

function expectSentences(report: JsonObject, chosen: boolean): void {
	const pieces = selectionPieces(report, 'sentence');
	const expected = [
		...(chosen ? [[skillFile, skillText.indexOf('1. Chosen step guidance.')]] : []),
		[skillFile, skillText.indexOf('Standards-only guidance.')],
		[skillFile, skillText.indexOf('1. Overlapping guidance.')],
		[referenceFile, referenceText.indexOf('1. Reference guidance.')],
	].toSorted(
		(left, right) =>
			String(left[0]).localeCompare(String(right[0])) || Number(left[1]) - Number(right[1]),
	);
	equal(
		pieces.map((piece) => {
			const where = objectValue(piece['where']);
			return [where['file'], objectValue(where['span'])['byte_start']];
		}),
		expected,
		'Union must include both containers, exclude outside paragraphs and keep overlap once',
	);
	for (const piece of pieces) {
		const where = objectValue(piece['where']);
		const text = where['file'] === skillFile ? skillText : referenceText;
		const span = objectValue(where['span']);
		equal(
			piece['text'],
			Buffer.from(text)
				.subarray(numberValue(span['byte_start']), numberValue(span['byte_end']))
				.toString('utf8'),
			'Sentence text/range drifted from fixture',
		);
	}
	const steps = selectionPieces(report, 'step');
	equal(
		steps.filter((piece) => objectValue(piece['where'])['file'] === referenceFile).length,
		0,
		'Reference must exercise a plain container without chosen steps',
	);
	equal(
		steps.filter((piece) => objectValue(piece['where'])['file'] === skillFile).length,
		chosen ? 2 : 0,
		'Chosen/not-chosen step control did not reach the required state',
	);
	expectUnionCounts(report, chosen);
}

export async function observeWithinKinds(options: {
	observer: RulebookProcess;
	evidence: string;
	sourceRoot: string;
}): Promise<void> {
	const { observer, evidence, sourceRoot } = options;
	const root = copySelectionFixture({
		destination: path.join(evidence, 'within-union'),
		sourceRoot,
		files: [skillFile, referenceFile],
	});
	writeChecked(root, skillFile, skillText);
	writeChecked(root, referenceFile, referenceText);
	filesystem.writeFileSync(
		path.join(root, 'evidence/rules/within-union.yml'),
		'id: within-standards\nlanguage: Markdown\nrule: {kind: section, has: {kind: atx_heading, regex: "^## Standards", stopBy: neighbor}}\n---\nid: within-sentence\nlanguage: Markdown\nrule: {any: [{kind: list_item}, {kind: paragraph, regex: "^(Outside|Standards-only)"}]}\n',
	);
	configure(root, (config) => {
		config['decision_model'] = {
			...objectValue(config['decision_model']),
			endpoint: observer.endpoint,
		};
		const evidenceConfig = objectValue(config['evidence']);
		const prior = objectValue(evidenceConfig['kinds']);
		const workflow = prior['workflow_section'];
		requireCondition(workflow, 'Canonical selected workflow kind missing');
		evidenceConfig['kinds'] = {
			workflow_section: workflow,
			step: { rule: 'step', within: ['workflow_section'], fields: { body: { from: 'remainder' } } },
			standards_section: { rule: 'within-standards', fields: { body: { from: 'remainder' } } },
			sentence: {
				rule: 'within-sentence',
				within: ['step', 'standards_section'],
				fields: { body: { from: 'remainder' } },
			},
		};
	});
	writeJson(path.join(root, 'rules.json'), {
		K001: {
			name: 'union_sentence_body',
			evidence: 'sentence',
			decision: {
				start: 'body',
				steps: {
					body: {
						mechanical_check: { tool: 'regex', field: 'body', pattern: '\\S' },
						yes: 'pass',
						['no']: { finding: 'A contained sentence must have a body.' },
					},
				},
			},
		},
	});
	const checked = await observer.run({
		root,
		name: 'within-union-check',
		arguments: ['lint', '--config', configName, '--check'],
		expectedExit: 0,
	});
	equal(checked.captures.length, 0, 'Within listing must not ask or need a key');
	const rows = arrayValue(jsonValueSchema.parse(JSON.parse(checked.stdout.toString('utf8')))).map(
		objectValue,
	);
	const sentence = rows.find((row) => row['kind'] === 'sentence');
	requireCondition(sentence, 'Contained kind missing from check listing');
	equal(
		sentence['within'],
		['step', 'standards_section'],
		'Check listing must preserve every container in config order',
	);
	await runSequence([true, false], async (chosen) => {
		observer.reply = scriptedSelectionReply({
			probability: (file) => (file === skillFile && chosen ? 0.93 : 0.12),
		});
		const name = chosen ? 'chosen' : 'not-selected';
		const listed = await observer.report({
			root,
			name: 'within-union-' + name,
			arguments: ['lint', '--config', configName, '--evidence', '--all'],
			expectedExit: 0,
			credential: true,
		});
		expectSentences(listed.report, chosen);
		const run = await observer.report({
			root,
			name: 'within-union-lint-' + name,
			arguments: ['lint', '--config', configName, '--format', 'json', '--all'],
			expectedExit: 0,
			credential: true,
		});
		const results = arrayValue(run.report['results']).map(objectValue);
		equal(
			results.map((result) => [result['file'], result['first_line'], result['outcome']]),
			[
				...(chosen ? [[skillFile, 7, 'pass']] : []),
				[skillFile, 11, 'pass'],
				[skillFile, 13, 'pass'],
				[referenceFile, 5, 'pass'],
			].toSorted(
				(left, right) =>
					String(left[0]).localeCompare(String(right[0])) || Number(left[1]) - Number(right[1]),
			),
			'Each admitted sentence must produce exactly one lint result, including overlap',
		);
		expectUnionCounts(run.report, chosen);
	});
}
