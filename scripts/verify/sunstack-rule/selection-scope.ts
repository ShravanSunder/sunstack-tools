/** R35/R39 real Git and staged dispatch observations. */
import filesystem from 'node:fs';
import path from 'node:path';

import {
	arrayValue,
	baselineGit,
	configName,
	configureEndpoint,
	equal,
	gitCommand,
	objectValue,
	requireCondition,
	runSequence,
} from './rulebook-fixture-support.ts';
import { type RulebookProcess } from './rulebook-process.ts';
import { selectionEntries, selectionPieces, selectionState } from './selection-checks.ts';
import {
	askingStepRule,
	copySelectionFixture,
	researchFile,
	presentationFile,
	writeChecked,
	changeSelectionKinds,
	keepKinds,
} from './selection-fixture.ts';
import { scriptedSelectionReply, selectionArguments } from './selection-reply.ts';
export async function observeSelectionChanges(options: {
	observer: RulebookProcess;
	evidence: string;
	sourceRoot: string;
}): Promise<void> {
	const { observer, evidence, sourceRoot } = options;
	await observeKindFields(options);
	const root = copySelectionFixture({
		destination: path.join(evidence, 'changed-hunk'),
		sourceRoot,
	});
	configureEndpoint(root, observer.endpoint);
	baselineGit(root);
	const file = path.join(root, researchFile);
	filesystem.writeFileSync(
		file,
		filesystem.readFileSync(file, 'utf8').replace('1. Frame one', '1. Frame changed one'),
	);
	observer.reply = scriptedSelectionReply({ probability: () => 0.93 });
	const result = await observer.report({
		root,
		name: 'selection-changed-hunk',
		arguments: ['lint', '--config', configName, '--evidence', '--since', 'HEAD'],
		expectedExit: 0,
		credential: true,
	});
	requireCondition(
		selectionEntries(result.report).length > 0 &&
			selectionEntries(result.report).every((entry) => entry.where.file === researchFile),
		'Changed hunk asked unrelated files',
	);
	requireCondition(
		result.captures.every((capture) => selectionState(capture.body).file === researchFile),
		'Unchanged file dispatched',
	);
	const files = arrayValue(result.report['file_counts']).map(objectValue);
	const silent = files.find((count) => count['file'] === presentationFile);
	requireCondition(silent, 'Untouched file absent from counts');
	const selection = objectValue(
		objectValue(objectValue(silent['kinds'])['workflow_section'])['selection'],
	);
	equal(selection['asked'], 0, 'Untouched candidates were asked');
	requireCondition(Number(selection['candidates']) > 0, 'Untouched candidate count omitted');
	await runSequence([false, true], async (absolute) => {
		const targets = copySelectionFixture({
			destination: path.join(evidence, absolute ? 'target-absolute' : 'target-only'),
			sourceRoot,
			files: ['skills/loader/SKILL.md', 'skills/target/SKILL.md'],
		});
		const targetPath = path.join(targets, 'skills/target/SKILL.md');
		const targetName = absolute ? targetPath : '../target/SKILL.md';
		writeChecked(
			targets,
			'skills/loader/SKILL.md',
			`# Loader\n\n## Workflow\n\n1. Inspect inputs.\n\n   load \`${targetName}\` and return its observations.\n\n2. Record results.\n`,
		);
		writeChecked(
			targets,
			'skills/target/SKILL.md',
			'---\ndescription: Use when checking a target.\n---\n',
		);
		configureEndpoint(targets, observer.endpoint);
		baselineGit(targets);
		filesystem.appendFileSync(
			path.join(targets, 'skills/target/SKILL.md'),
			'\nChanged target, no loader hunk.\n',
		);
		const loaded = await observer.report({
			root: targets,
			name: absolute ? 'selection-absolute-target-modified' : 'selection-target-only',
			arguments: ['lint', '--config', configName, '--evidence', '--since', 'HEAD'],
			expectedExit: 0,
			credential: true,
		});
		requireCondition(
			loaded.captures.some(
				(capture) => selectionState(capture.body).file === 'skills/loader/SKILL.md',
			),
			'Target-only change did not select loading file',
		);
		const loads = selectionPieces(loaded.report, 'load', 'skills/loader/SKILL.md');
		requireCondition(loads.length === 1, 'Target change failed to pick dependent load');
		equal(
			objectValue(objectValue(loads[0]?.['fields'])['step'])['values'],
			['1. Inspect inputs.'],
			'Load context is not chosen step',
		);
		filesystem.unlinkSync(path.join(targets, 'skills/target/SKILL.md'));
		equal(
			gitCommand(targets, ['diff', '--name-status', 'HEAD']).trim(),
			'D\tskills/target/SKILL.md',
			'Target-deletion fixture must change only the target, with no loader hunk',
		);
		const deleted = await observer.report({
			root: targets,
			name: absolute ? 'selection-absolute-target-deleted' : 'selection-target-deleted',
			arguments: ['lint', '--config', configName, '--evidence', '--since', 'HEAD'],
			expectedExit: 0,
			credential: true,
		});
		requireCondition(
			deleted.captures.some(
				(capture) => selectionState(capture.body).file === 'skills/loader/SKILL.md',
			),
			'Deleted target did not select loading file',
		);
		requireCondition(
			selectionEntries(deleted.report).some(
				(entry) =>
					entry.where.file === 'skills/loader/SKILL.md' && entry.outcome.kind === 'selected',
			),
			'Deleted target loader has no selected candidate outcome',
		);
		const deletedLoads = selectionPieces(deleted.report, 'load', 'skills/loader/SKILL.md');
		equal(deletedLoads.length, 1, 'Deleted target failed to pick dependent load');
		const deletedTargets = arrayValue(
			objectValue(objectValue(deletedLoads[0]?.['fields'])['targets'])['values'],
		).map(objectValue);
		equal(deletedTargets.length, 1, 'Deleted target identity missing from dependent load');
		equal(
			deletedTargets[0]?.['path'],
			absolute ? targetPath : 'skills/loader/../target/SKILL.md',
			'Missing target must preserve its serialized unresolved path',
		);
		equal(deletedTargets[0]?.['exists'], false, 'Deleted target must report exists:false');
	});
	const staged = copySelectionFixture({
		destination: path.join(evidence, 'selection-barrier'),
		sourceRoot,
	});
	configureEndpoint(staged, observer.endpoint);
	askingStepRule(staged);
	const start = observer.events.length;
	const before = observer.captures.length;
	const ordered = await observer.report({
		root: staged,
		name: 'selection-before-rules',
		arguments: selectionArguments('--all'),
		expectedExit: 0,
		credential: true,
	});
	const requests = ordered.captures;
	const firstRule = requests.findIndex(
		(capture) => !('candidates' in objectValue(capture.body['state'])),
	);
	requireCondition(firstRule > 0, 'Barrier fixture did not reach selection and rule requests');
	const events = observer.events.slice(start);
	const firstEvent = events.findIndex(
		(event) => event.event === 'received' && event.request === before + firstRule + 1,
	);
	for (const [index, capture] of requests.entries())
		if ('candidates' in objectValue(capture.body['state']))
			requireCondition(
				events.findIndex(
					(event) => event.event === 'responded' && event.request === before + index + 1,
				) < firstEvent,
				'Rule dispatched before every selection completed',
			);
}

async function observeKindFields(options: {
	observer: RulebookProcess;
	evidence: string;
	sourceRoot: string;
}): Promise<void> {
	const { observer, evidence, sourceRoot } = options;
	const root = copySelectionFixture({
		destination: path.join(evidence, 'kind-fields'),
		sourceRoot,
		files: ['skills/custom/SKILL.md'],
		allKinds: true,
	});
	keepKinds(root, ['workflow_section', 'step', 'section_steps']);
	const text =
		'---\ndescription: Use when checking fields.\n---\n\n## Parent\n\n1. Parent item.\n\n### Chosen child\n\n1. Child one.\n2. Child two.\n';
	writeChecked(root, 'skills/custom/SKILL.md', text);
	changeSelectionKinds(root, (kinds) => {
		kinds['overview'] = {
			rule: 'skill',
			fields: { sections: { from: 'same_file', kind: 'workflow_section' } },
		};
		kinds['parent_view'] = {
			rule: 'section',
			fields: {
				heading: { from: 'first_line' },
				body: { from: 'remainder', exclude: { rule: 'section' } },
			},
		};
	});
	configureEndpoint(root, observer.endpoint);
	observer.reply = scriptedSelectionReply({
		probability: (_file, key) => (key === 'c2' ? 0.93 : 0.12),
	});
	const result = await observer.report({
		root,
		name: 'kind-fields-use-chosen-pieces',
		arguments: ['lint', '--config', configName, '--evidence', '--all'],
		expectedExit: 0,
		credential: true,
	});
	const entries = selectionEntries(result.report);
	equal(
		entries.map((entry) => entry.outcome.kind),
		['not_selected', 'selected'],
		'Containment fixture must choose only child',
	);
	const selected = selectionPieces(result.report, 'step');
	equal(selected.length, 2, 'Parent raw item leaked into contained steps');
	const first = text.indexOf('1. Child one.');
	const second = text.indexOf('2. Child two.');
	const expected = [text.slice(first, second), text.slice(second)];
	const parents = selectionPieces(result.report, 'section_steps').filter(
		(piece) => objectValue(piece['where'])['first_line'] === 5,
	);
	equal(parents.length, 1, 'Missing parent section');
	equal(
		objectValue(objectValue(parents[0]?.['fields'])['steps'])['values'],
		expected,
		'Inside.kind returned raw matches rather than exactly chosen-step texts',
	);
	const overview = selectionPieces(result.report, 'overview');
	equal(overview.length, 1, 'Missing same-file witness');
	equal(
		objectValue(objectValue(overview[0]?.['fields'])['sections'])['values'],
		['### Chosen child'],
		'Same-file.kind exposed unchosen parent',
	);
	const parent = selectionPieces(result.report, 'parent_view').find(
		(piece) => objectValue(piece['where'])['first_line'] === 5,
	);
	requireCondition(parent, 'Missing remainder parent');
	equal(
		objectValue(objectValue(parent['fields'])['body'])['value'],
		'\n1. Parent item.\n\n',
		'Remainder.exclude erased self or kept subsection text',
	);
}
