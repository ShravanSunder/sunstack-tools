/** U13–U16 lasting binary journey, selected by rulebook-lint --case selection. */
import filesystem from 'node:fs';
import path from 'node:path';

import {
	configName,
	configureEndpoint,
	equal,
	numberValue,
	objectValue,
	requireCondition,
	runSequence,
	textValue,
	writeJson,
	type JourneyArguments,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import {
	requireCandidateKeys,
	requireSameShared,
	requireSelectionCounts,
	requireSelectionRequest,
	selectionEntries,
	selectionPieces,
	selectionState,
} from './selection-checks.ts';
import {
	copySelectionFixture,
	researchFile,
	presentationFile,
	emptyFile,
	manyFile,
	oracleRows,
	selectedQuestion,
	selectionKinds,
	changeSelectionKinds,
	keepKinds,
} from './selection-fixture.ts';
import { observeSelectionLimits } from './selection-limits.ts';
import { scriptedSelectionReply, selectionArguments } from './selection-reply.ts';
import { observeSelectionChanges } from './selection-scope.ts';

function removeRanges(
	text: Buffer,
	start: number,
	end: number,
	ranges: readonly { start: number; end: number }[],
): string {
	let cursor = start;
	const chunks: Buffer[] = [];
	for (const range of ranges.toSorted(
		(left, right) => left.start - right.start || left.end - right.end,
	)) {
		if (range.start > cursor) chunks.push(text.subarray(cursor, range.start));
		cursor = Math.max(cursor, range.end);
	}
	if (cursor < end) chunks.push(text.subarray(cursor, end));
	return Buffer.concat(chunks).toString('utf8');
}
export function expectedSelectionState(
	root: string,
	file: string,
): { candidates: JsonObject; shared: JsonObject } {
	const bytes = filesystem.readFileSync(path.join(root, file));
	const candidates: JsonObject = {};
	const sections = oracleRows(root, 'section', file);
	const matches = oracleRows(root, 'workflow-candidate', file);
	for (const [index, match] of matches.entries()) {
		const start = numberValue(match['start']);
		const end = numberValue(match['end']);
		const text = bytes.subarray(start, end).toString('utf8');
		equal(text, match['text'], 'S0 source differs from ast-grep oracle');
		const length = Buffer.byteLength(text.split('\n')[0] ?? '') + (text.includes('\n') ? 1 : 0);
		const taken = [
			{ start, end: start + length },
			...sections
				.filter((row) => numberValue(row['start']) > start && numberValue(row['end']) <= end)
				.map((row) => ({ start: numberValue(row['start']), end: numberValue(row['end']) })),
		];
		candidates[`c${index + 1}`] = {
			heading: (text.split('\n')[0] ?? '').trimEnd(),
			body: removeRanges(bytes, start, end, taken),
		};
	}
	return {
		candidates,
		shared: {
			outline: sections.map((row) => (textValue(row['text']).split('\n')[0] ?? '').trimEnd()),
		},
	};
}

function checkReport(root: string, report: JsonObject, files: readonly string[]): void {
	const kinds = selectionKinds(root);
	requireSelectionCounts({
		report,
		files,
		kinds: Object.keys(kinds),
		selectedKinds: Object.keys(kinds).filter((name) => 'select' in objectValue(kinds[name])),
	});
}
function requireResearchSteps(report: JsonObject, count: number): void {
	const pieces = selectionPieces(report, 'step', researchFile);
	equal(pieces.length, count, 'Wrong number of contained research steps');
	if (count)
		equal(
			pieces.map((piece) => objectValue(piece['where'])['first_line']),
			[21, 22, 24, 25, 26, 27],
			'Steps are not the adopted six items',
		);
}
async function observeWorked(observer: RulebookProcess, fixtures: string): Promise<void> {
	await runSequence(
		[
			{ name: 'selected', probability: 0.93, exit: 0, outcome: 'selected', steps: 6 },
			{ name: 'not-selected', probability: 0.12, exit: 0, outcome: 'not_selected', steps: 0 },
			{ name: 'uncertain', probability: 0.5, exit: 5, outcome: 'uncertain', steps: 0 },
		],
		async (scenario) => {
			const root = path.join(fixtures, scenario.name);
			configureEndpoint(root, observer.endpoint);
			observer.reply = scriptedSelectionReply({
				probability: (file, key) =>
					file === researchFile && key === 'c1' ? scenario.probability : 0.12,
			});
			const result = await observer.report({
				root,
				name: scenario.name,
				arguments: ['lint', '--config', configName, '--evidence', '--all'],
				expectedExit: scenario.exit,
				credential: true,
			});
			checkReport(root, result.report, [researchFile, presentationFile, emptyFile]);
			requireResearchSteps(result.report, scenario.steps);
			const research = selectionEntries(result.report).filter(
				(entry) => entry.kind === 'workflow_section' && entry.where.file === researchFile,
			);
			equal(research.length, 1, 'Research candidate split or lost');
			equal(research[0]?.outcome.kind, scenario.outcome, 'Wrong selection outcome');
			for (const capture of result.captures) {
				const state = selectionState(capture.body);
				const expected = expectedSelectionState(root, state.file);
				requireSelectionRequest({
					body: capture.body,
					file: state.file,
					question: selectedQuestion(root),
					sharedNames: ['outline'],
					...expected,
				});
			}
			equal(result.captures.length, 2, 'One selection request per candidate-bearing file expected');
			if (scenario.name === 'selected') {
				const repeated = await observer.report({
					root,
					name: 'selected-repeat',
					arguments: ['lint', '--config', configName, '--evidence', '--all'],
					expectedExit: 0,
					credential: true,
				});
				equal(repeated.report, result.report, 'Repeat selection report bytes/semantics changed');
				equal(
					repeated.captures.map((capture) => capture.raw),
					result.captures.map((capture) => capture.raw),
					'Repeat request order/body changed',
				);
			}
		},
	);
	const root = path.join(fixtures, 'default-format');
	configureEndpoint(root, observer.endpoint);
	observer.reply = scriptedSelectionReply({ probability: () => 0.12 });
	const rendered = await observer.run({
		root,
		name: 'selection-default',
		arguments: ['lint', '--config', configName, '--all'],
		expectedExit: 0,
		credential: true,
	});
	const output = rendered.stdout.toString('utf8');
	requireCondition(
		output.includes('workflow_section') &&
			output.includes(presentationFile) &&
			/not.selected|not_selected/.test(output),
		'Default format hides unselected file/counters',
	);
	const gated = path.join(fixtures, 'no-key');
	const missing = await observer.report({
		root: gated,
		name: 'selected-no-key',
		arguments: selectionArguments('--evidence', '--all'),
		expectedExit: 2,
	});
	equal(missing.captures.length, 0, 'No-key selection dispatched');
	equal(
		objectValue(missing.report['usage'])['checked_files_acquired'],
		0,
		'No-key selection acquired checked content',
	);
	equal(objectValue(missing.report['status'])['kind'], 'invalid', 'Missing key must be invalid');
	const plain = path.join(fixtures, 'plain');
	const listing = await observer.report({
		root: plain,
		name: 'plain-no-key',
		arguments: ['lint', '--config', configName, '--evidence', '--all'],
		expectedExit: 0,
	});
	equal(listing.captures.length, 0, 'Plain evidence asked model');
	requireCondition(
		selectionPieces(listing.report, 'step').length > 0,
		'Plain no-key witness selected nothing',
	);
}
async function observeBoundaries(observer: RulebookProcess, fixtures: string): Promise<void> {
	await runSequence([0.29, 0.3, 0.31, 0.69, 0.7, 0.71], async (probability) => {
		const name = `threshold-${probability}`;
		const root = path.join(fixtures, name);
		configureEndpoint(root, observer.endpoint);
		observer.reply = scriptedSelectionReply({ probability: () => probability });
		const result = await observer.report({
			root,
			name,
			arguments: ['lint', '--config', configName, '--evidence', '--all'],
			expectedExit: probability > 0.3 && probability < 0.7 ? 5 : 0,
			credential: true,
		});
		const entries = selectionEntries(result.report);
		equal(entries.length, 1, 'Boundary fixture must reach one candidate');
		equal(
			entries[0]?.outcome.kind,
			probability >= 0.7 ? 'selected' : probability <= 0.3 ? 'not_selected' : 'uncertain',
			'Threshold equality routed incorrectly',
		);
	});
	const root = path.join(fixtures, 'many');
	configureEndpoint(root, observer.endpoint);
	observer.reply = scriptedSelectionReply({
		probability: (_file, key) => (Number(key.slice(1)) % 2 ? 0.93 : 0.12),
	});
	const result = await observer.report({
		root,
		name: 'thirty-three',
		arguments: ['lint', '--config', configName, '--evidence', '--all'],
		expectedExit: 0,
		credential: true,
	});
	requireCandidateKeys({
		bodies: result.captures.map((capture) => capture.body),
		groups: [Array.from({ length: 32 }, (_value, index) => `c${index + 1}`), ['c33']],
	});
	requireSameShared(result.captures.map((capture) => capture.body));
	equal(selectionEntries(result.report).length, 33, 'Lost candidate at question-count split');
	checkReport(root, result.report, [manyFile]);
}
export function prepareSelectionJourney(options: { evidence: string; sourceRoot: string }): void {
	filesystem.mkdirSync(options.evidence);
	const fixtures = path.join(options.evidence, 'fixtures');
	filesystem.mkdirSync(fixtures);
	for (const name of [
		'selected',
		'not-selected',
		'uncertain',
		'default-format',
		'no-key',
		'plain',
		'many',
		...Array.from([0.29, 0.3, 0.31, 0.69, 0.7, 0.71], (value) => `threshold-${value}`),
	]) {
		const root = copySelectionFixture({
			destination: path.join(fixtures, name),
			sourceRoot: options.sourceRoot,
			...(name === 'many'
				? { files: [manyFile] }
				: name.startsWith('threshold')
					? { files: [researchFile] }
					: {}),
		});
		if (name === 'plain')
			changeSelectionKinds(root, (kinds) => {
				delete objectValue(kinds['workflow_section'])['select'];
				delete objectValue(kinds['step'])['within'];
			});
		if (name === 'many') keepKinds(root, ['workflow_section']);
	}
	writeJson(path.join(options.evidence, 'prepared.json'), {
		product_observed: false,
		oracle: 'docs/specs/2026-10-30-evidence-selection/verification-design.md',
		source_root: options.sourceRoot,
		remaining_native_joins: [
			'U14 staged-builder dependency tests',
			'U17 ast-grep oracle and standard utils',
			'R40 zero checked-source structural proof',
		],
	});
}
export async function runSelectionJourney(options: JourneyArguments): Promise<void> {
	const target = options.prepare
		? undefined
		: declaredTarget({
				binary: options.binary ?? '',
				source: options['source-root'],
				commit: options.commit ?? '',
				...(options.sha256 ? { digest: options.sha256 } : {}),
			});
	const evidence = path.resolve(options.evidence);
	prepareSelectionJourney({ evidence, sourceRoot: options['source-root'] });
	if (options.prepare) {
		process.stdout.write('Prepared selection journey; product unobserved\n');
		return;
	}
	requireCondition(target, 'Missing declared selection target');
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		await observeWorked(observer, path.join(evidence, 'fixtures'));
		await observeBoundaries(observer, path.join(evidence, 'fixtures'));
		await observeSelectionChanges({ observer, evidence, sourceRoot: options['source-root'] });
		await observeSelectionLimits({ observer, evidence, sourceRoot: options['source-root'] });
	} finally {
		await observer.close();
	}
	writeJson(path.join(evidence, 'pass.json'), {
		source_commit: target.commit,
		binary_sha256: target.digest,
		rows: ['U13', 'U14-binary', 'U15', 'U16-binary'],
		unverified: [
			'U17/library joins need inspected native proof',
			'compatibility probe separately required',
			'real model and V21 Lead-owned',
		],
	});
	process.stdout.write(
		'Selection journey passed on declared binary; native/compatibility/live joins remain separate\n',
	);
}
