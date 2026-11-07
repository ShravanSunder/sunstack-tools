/** O6 selected aliases stay unjudged; every symlink target remains inside this owned fixture. */
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import { requireBars, requireCasesReport, ordinaryProjection } from './rulebook-case-checks.ts';
import { casesConfigName, prepareCasesFixture } from './rulebook-case-fixtures.ts';
import { includedFolder } from './rulebook-filters.ts';
import {
	arrayValue,
	equal,
	gitCommand,
	objectValue,
	parseObject,
	requireCondition,
	runSequence,
	textValue,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import { type RulebookProcess } from './rulebook-process.ts';

const leafAlias = `${includedFolder}/notes/alias.md`;
const ancestorAlias = `${includedFolder}/linked-notes`;
const folderAlias = 'plugins/one/skills/root-alias';

export function prepareSymlinkCasesFixture(root: string): JsonObject {
	const prepared = prepareCasesFixture(root);
	filesystem.symlinkSync('one.md', path.join(root, leafAlias));
	filesystem.symlinkSync('notes', path.join(root, ancestorAlias));
	filesystem.symlinkSync('../../..', path.join(root, folderAlias));
	const description = 'Root alias folder guidance.';
	const opening = 'Alias root opening.';
	filesystem.writeFileSync(
		path.join(root, 'overview.md'),
		`---\ndescription: ${description}\nopening: ${opening}\n---\n`,
	);
	const canonicalRoot = filesystem.realpathSync(root);
	for (const alias of [leafAlias, ancestorAlias, folderAlias])
		requireCondition(
			filesystem.realpathSync(path.join(root, alias)).startsWith(`${canonicalRoot}/`) ||
				filesystem.realpathSync(path.join(root, alias)) === canonicalRoot,
			'Symlink fixture target left its owned repository',
		);
	gitCommand(root, ['add', '.']);
	gitCommand(root, ['commit', '-qm', 'Historical selected aliases stay within fixture root']);
	const revision = gitCommand(root, ['rev-parse', 'HEAD']);
	const rule = arrayValue(prepared['rule_cases'])
		.map(objectValue)
		.find((definition) => definition['id'] === 'current-pass');
	const filter = arrayValue(prepared['filter_cases'])
		.map(objectValue)
		.find((definition) => definition['id'] === 'filter-right');
	requireCondition(rule && filter, 'Missing positive symlink control definitions');
	const common = { revision, set: 'held-out' };
	return {
		...prepared,
		alias_revision: revision,
		alias_cases: [
			{ ...rule, ...common, id: 'selected-leaf-alias', file: leafAlias },
			{ ...rule, ...common, id: 'selected-ancestor-alias', file: `${ancestorAlias}/one.md` },
			{
				...filter,
				...common,
				id: 'selected-folder-alias',
				folder: folderAlias,
				reads_sha256: createHash('sha256')
					.update(JSON.stringify({ description, opening }))
					.digest('hex'),
			},
		],
		normal_case: { ...rule, ...common, id: 'selected-normal-control' },
	};
}

export async function observeSymlinkCasePaths(
	observer: RulebookProcess,
	root: string,
	prepared: JsonObject,
	evidence: string,
): Promise<void> {
	const development = [
		...arrayValue(prepared['rule_cases']),
		...arrayValue(prepared['filter_cases']),
	].map(objectValue);
	const aliases = arrayValue(prepared['alias_cases']).map(objectValue);
	const normal = objectValue(prepared['normal_case']);
	let normalBodies: JsonObject[] = [];
	let normalReport: JsonObject = {};
	await runSequence(['normal', 'only', 'mixed'] as const, async (phase) => {
		const selected =
			phase === 'normal' ? [normal] : phase === 'only' ? aliases : [...aliases, normal];
		const expected: Record<string, string> = {};
		for (const definition of selected)
			expected[textValue(definition['id'])] =
				definition['id'] === normal['id'] ? 'right' : 'not_judged';
		const input = path.join(evidence, `symlink-${phase}-cases.json`);
		writeJson(input, {
			repository: root,
			cases: [...development, ...selected],
			bars: prepared['bars'],
		});
		const result = await observer.run({
			root,
			name: `symlink-${phase}`,
			arguments: ['lint', '--config', casesConfigName, 'cases', input, '--set', 'held-out'],
			credential: true,
			expectedExit: phase === 'normal' ? 0 : 1,
		});
		const report = parseObject(result.stdout.toString('utf8'));
		requireCasesReport({
			report,
			definitions: selected,
			expected,
			notSelected: development.map((definition) => textValue(definition['id'])),
			expectedErrors:
				phase === 'only'
					? {
							[textValue(prepared['alias_revision'])]:
								'no judgeable selected case paths at this revision',
						}
					: {},
		});
		requireBars(report, false);
		for (const row of arrayValue(report['cases']).map(objectValue))
			if (row['id'] !== normal['id'])
				equal(
					row['outcome'],
					{ kind: 'not_judged', reason: 'selected path is a symlink' },
					'Selected alias was judged or rebound',
				);
		equal(
			result.captures.length,
			phase === 'only' ? 0 : 2,
			'Selected alias spent a model request or suppressed the regular control',
		);
		const runs = arrayValue(report['runs']).map(objectValue);
		if (phase === 'only') equal(runs.length, 0, 'Alias-only selection started ordinary lint');
		if (phase === 'normal') {
			equal(runs.length, 1, 'Regular control lost its revision run');
			normalBodies = result.captures.map((capture) => capture.body);
			normalReport = objectValue(runs[0]?.['report']);
		}
		if (phase === 'mixed') {
			equal(runs.length, 1, 'Mixed aliases added revision runs');
			equal(
				result.captures.map((capture) => capture.body),
				normalBodies,
				'Alias inputs changed the regular selected request',
			);
			equal(
				ordinaryProjection(objectValue(runs[0]?.['report'])),
				ordinaryProjection(normalReport),
				'Aliases changed regular lint results',
			);
		}
	});
}
