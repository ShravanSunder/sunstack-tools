// Keyless calls through the real addon: snapshots, check effects and run endings.
import assert from 'node:assert/strict';
import { readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

import type { Configuration, LoadedConfiguration, RuleEntry, RunOutcome } from '../src/index.ts';
import { check, load, run } from '../src/index.ts';
import type { ControlledBackend } from './support/controlled-backend.ts';
import { startControlledBackend } from './support/controlled-backend.ts';
import type { OwnedDirectory } from './support/sdk-fixtures.ts';
import {
	inlineConfiguration,
	ownedDirectory,
	writeFiles,
	writeSkillRepository,
} from './support/sdk-fixtures.ts';

// This process has no credential and no credential store.
delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

const ruleA = '^Use when';
const ruleB = '^Wrong';

function mechanicalEntries(pattern: string): Readonly<Record<string, RuleEntry>> {
	return {
		K001: {
			name: 'description',
			evidence: 'skill',
			severity: 'error',
			decision: {
				start: 'starts',
				steps: {
					starts: {
						mechanical_check: { tool: 'regex', field: 'description', pattern },
						yes: 'pass',
						no: { finding: 'Description must begin with Use when.', at: 'description' },
					},
				},
			},
		},
	};
}

function writeMechanicalProject(definitions: string, checked: string): void {
	writeFiles(checked, { 'docs/one.md': '---\ndescription: Wrong opening\n---\n\n# One\n' });
	writeFiles(definitions, {
		'lint/sunstack.lint.json': JSON.stringify({
			rulebook: [{ markdown: 'standards/rulebook.md', rules: 'standards/rules.json' }],
			marker_prefix: 'K',
			check: { files: ['docs/*.md'] },
			evidence: {
				ast_grep: 'evidence/sgconfig.yml',
				kinds: {
					skill: { rule: 'skill', fields: { description: { from: 'yaml', key: 'description' } } },
				},
			},
			decision_model: { model: 'jev-latest' },
		}),
		'lint/standards/rulebook.md': 'Description must begin with Use when. <!-- K001 -->\n',
		'lint/standards/rules.json': JSON.stringify(mechanicalEntries(ruleA)),
		'lint/evidence/sgconfig.yml': 'ruleDirs: [rules]\n',
		'lint/evidence/rules/nested/skill.yml':
			'id: skill\nlanguage: md\nrule: {matches: frontmatter}\n',
	});
}

function findingCount(outcome: RunOutcome): number {
	assert.equal(
		outcome.kind,
		'completed',
		JSON.stringify(outcome, (_key, value: unknown) =>
			typeof value === 'bigint' ? value.toString() : value,
		),
	);
	return outcome.kind === 'completed' ? outcome.report.findings.length : -1;
}

// Tests run in order; these fixtures are released after the last one.
const definitions: OwnedDirectory = ownedDirectory('definitions');
const checked: OwnedDirectory = ownedDirectory('checked');
const backend: ControlledBackend = await startControlledBackend();

async function loaded(directory: string): Promise<LoadedConfiguration> {
	const outcome = await load({ directory });
	assert.equal(outcome.kind, 'loaded', JSON.stringify(outcome));
	if (outcome.kind !== 'loaded') throw new Error('unreachable');
	return outcome.value;
}

await test('a held snapshot, an edited copy and a reload each execute their own definitions', async () => {
	writeMechanicalProject(definitions.path, checked.path);
	const project = join(definitions.path, 'lint');
	const held = await loaded(join(project, 'standards'));
	assert.ok(Object.isFrozen(held.configuration), 'loaded values are frozen');
	assert.ok(
		Object.isFrozen(held.configuration.rulebooks[0]?.entries.value),
		'nested values are frozen',
	);

	const rulesFile = join(project, 'standards', 'rules.json');
	writeFileSync(rulesFile, JSON.stringify(mechanicalEntries(ruleB)));
	const root = checked.path;

	assert.equal(
		findingCount(await run(held, { root, scope: { kind: 'all' } })),
		1,
		'held A executes A',
	);

	const rulebook = held.configuration.rulebooks[0];
	assert.ok(rulebook !== undefined);
	const copy: Configuration = {
		...held.configuration,
		rulebooks: [{ ...rulebook, entries: { ...rulebook.entries, value: mechanicalEntries(ruleB) } }],
	};
	const edited = await run(copy, { root, scope: { kind: 'all' } });
	assert.equal(findingCount(edited), 0, 'the edited copy executes its edit');
	if (edited.kind === 'completed') {
		const editedSource = edited.report.sources.find((source) => source.origin.kind === 'edited');
		assert.ok(editedSource !== undefined, 'the edit is visibly edited');
		assert.equal(editedSource.representation, 'normalized_value');
		assert.match(editedSource.content, /\^Wrong/u);
	}

	assert.equal(
		findingCount(await run(held, { root, scope: { kind: 'all' } })),
		1,
		'the copy did not change the held value',
	);
	assert.equal(
		findingCount(await run(await loaded(project), { root, scope: { kind: 'all' } })),
		0,
		'only reload sees B',
	);

	// Removing every definition file leaves the held snapshot runnable.
	rmSync(join(project, 'standards'), { recursive: true });
	rmSync(join(project, 'evidence'), { recursive: true });
	assert.equal(findingCount(await run(held, { root, scope: { kind: 'all' } })), 1);
	assert.equal((await load({ directory: project })).kind, 'unavailable');
	writeMechanicalProject(definitions.path, checked.path);
});

await test('the sources receipt is never executable input', async () => {
	writeMechanicalProject(definitions.path, checked.path);
	const held = await loaded(join(definitions.path, 'lint'));
	const rulesFile = join(definitions.path, 'lint', 'standards', 'rules.json');
	const tampered: LoadedConfiguration = {
		...held,
		sources: held.sources.map((source) =>
			Object.assign({}, source, {
				content: readFileSync(rulesFile, 'utf8').replace('Use when', 'Wrong'),
			}),
		),
	};
	assert.equal(
		findingCount(await run(tampered, { root: checked.path, scope: { kind: 'all' } })),
		1,
	);
});

await test('check admits an asking, filtered configuration with no key, request or checked root', async () => {
	const base = ownedDirectory('base');
	try {
		const outcome = await check(inlineConfiguration(base.path, backend.endpoint));
		assert.equal(outcome.kind, 'valid', JSON.stringify(outcome));
		if (outcome.kind === 'valid') {
			assert.deepEqual(outcome.summary, {
				rulebook_count: 1,
				active_rule_count: 1,
				skipped_rule_count: 0,
				evidence_kind_count: 1,
				filter_count: 1,
			});
		}
		assert.equal(backend.received.length, 0, 'check sends no provider request');
	} finally {
		base.remove();
	}
});

await test('run endings keep the exact engine status, diagnostics and checked paths', async () => {
	const base = ownedDirectory('base');
	const repository = ownedDirectory('repository');
	try {
		writeSkillRepository(repository.path);
		const configuration = inlineConfiguration(base.path, backend.endpoint);

		const missingKey = await run(configuration, { root: repository.path, scope: { kind: 'all' } });
		assert.equal(missingKey.kind, 'invalid');
		if (missingKey.kind === 'invalid') {
			assert.equal(missingKey.diagnostics[0]?.code, 'credential_missing');
			assert.equal(missingKey.report?.status.kind, 'invalid');
			assert.deepEqual(missingKey.diagnostics[0]?.origin, {
				kind: 'inline',
				id: 'instruction-standards',
				label: 'instruction-standards',
			});
		}

		const relative = await run(configuration, { root: 'relative/root', scope: { kind: 'all' } });
		assert.deepEqual(
			relative.kind === 'invalid' ? [relative.diagnostics[0]?.code, relative.report] : null,
			['unsupported_inputs', null],
		);

		// A keyless mechanical configuration reaches target selection without a credential.
		const mechanical: Configuration = {
			id: 'mechanical',
			base_directory: base.path,
			marker_prefix: 'K',
			check: { files: ['**/*.md'] },
			decision_model: { model: 'jev-latest' },
			evidence: {
				kind: 'inline',
				utilities: {},
				matchers: { skill: { language: 'Markdown', rule: { matches: 'frontmatter' } } },
				kinds: {
					skill: { rule: 'skill', fields: { description: { from: 'yaml', key: 'description' } } },
				},
			},
			rulebooks: [
				{
					kind: 'inline',
					id: 'mechanical-rules',
					markdown: 'Description must begin with Use when. <!-- K001 -->\n',
					entries: mechanicalEntries(ruleA),
				},
			],
		};
		writeFiles(repository.path, { 'outside.md': 'x\n' });
		const outside = await run(mechanical, {
			root: join(repository.path, 'skills'),
			paths: ['../outside.md'],
			scope: { kind: 'all' },
		});
		assert.equal(outside.kind, 'invalid');
		if (outside.kind === 'invalid') {
			const diagnostic = outside.diagnostics[0];
			assert.equal(diagnostic?.code, 'unsupported_inputs');
			assert.equal(diagnostic?.origin, null, 'a checked-path refusal has no definition origin');
			assert.equal(diagnostic?.path, '../outside.md');
		}
		const missingPath = await run(mechanical, {
			root: repository.path,
			paths: ['absent.md'],
			scope: { kind: 'all' },
		});
		assert.equal(
			missingPath.kind === 'invalid' ? missingPath.diagnostics[0]?.code : missingPath.kind,
			'path_not_found',
		);
		const nothing = await run(mechanical, {
			root: repository.path,
			paths: ['skills/alpha'],
			scope: { kind: 'all' },
		});
		assert.equal(
			nothing.kind === 'completed' ? nothing.report.status.kind : nothing.kind,
			'nothing_selected',
		);

		const changeScopeOnSubdirectory = await run(mechanical, {
			root: join(repository.path, 'skills'),
			scope: { kind: 'default' },
		});
		assert.equal(changeScopeOnSubdirectory.kind, 'invalid');
	} finally {
		base.remove();
		repository.remove();
	}
});

await backend.close();
definitions.remove();
checked.remove();
