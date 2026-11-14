// Exercise captured source receipts using only the fresh installed package.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import type { LoadedConfiguration } from '@sunstack/sdk';
import { check, deserialize, load, run, serialize } from '@sunstack/sdk';

const project = join(realpathSync(process.cwd()), 'receipt-project');
mkdirSync(join(project, 'rules'), { recursive: true });
const original = '# original project café\nruleDirs: [rules]\n';
const projectFile = join(project, 'sgconfig.yml');
const entries = {
	K001: {
		name: 'body',
		evidence: 'paragraph',
		severity: 'error',
		decision: {
			start: 'present',
			steps: {
				present: {
					mechanical_check: { tool: 'complete', field: 'body' },
					yes: 'pass',
					no: { finding: 'Missing body.' },
				},
			},
		},
	},
};
const files = {
	'sunstack.lint.json': JSON.stringify({
		rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
		marker_prefix: 'K',
		check: { files: ['checked.md'] },
		decision_model: { model: 'jev-latest' },
		evidence: {
			ast_grep: 'sgconfig.yml',
			kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
		},
	}),
	'sgconfig.yml': original,
	'rules/paragraph.yml': 'id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n',
	'rulebook.md': 'Body is complete. <!-- K001 -->\n',
	'rules.json': JSON.stringify(entries),
	'checked.md': '# Guide\n\nA paragraph.\n',
};
for (const [path, content] of Object.entries(files)) writeFileSync(join(project, path), content);
const outcome = await load({ directory: project });
assert.equal(outcome.kind, 'loaded');
if (outcome.kind !== 'loaded') throw new Error('load failed');
const held = outcome.value;
assert.deepEqual(
	held.sources.map((source) =>
		source.origin.kind === 'file' ? source.origin.path : source.origin.kind,
	),
	['sunstack.lint.json', 'sgconfig.yml', 'rules/paragraph.yml', 'rulebook.md', 'rules.json'].map(
		(path) => join(project, path),
	),
);
const projectSources = held.sources.filter(
	(source) => source.origin.kind === 'file' && source.origin.path === projectFile,
);
assert.equal(projectSources.length, 1);
assert.deepEqual(projectSources[0], {
	origin: { kind: 'file', path: projectFile },
	content: original,
	sha256: createHash('sha256').update(original).digest('hex'),
	representation: 'original_text',
	acquisition: 'read',
});
const saved = serialize({ kind: 'loaded_configuration', value: held });
assert.equal(saved.kind, 'serialized');
if (saved.kind !== 'serialized') throw new Error('save failed');
writeFileSync(projectFile, 'ruleDirs: [missing]\n');
rmSync(projectFile);
const restored = await deserialize(saved.json);
assert.equal(restored.kind, 'decoded');
if (restored.kind !== 'decoded' || restored.value.kind !== 'loaded_configuration')
	throw new Error('restore failed');
assert.deepEqual(restored.value.value, held);
assert.equal((await check(restored.value.value)).kind, 'valid');
const tampered: LoadedConfiguration = { ...restored.value.value, sources: [] };
const options = { root: project, scope: { kind: 'all' as const } };
const ordinary = await run(held, options);
const receiptOnly = await run(tampered, options);
assert.equal(ordinary.kind, 'completed');
assert.equal(receiptOnly.kind, 'completed');
if (ordinary.kind !== 'completed' || receiptOnly.kind !== 'completed')
	throw new Error('captured execution failed');
assert.equal(ordinary.report.status.kind, 'clean');
assert.deepEqual(receiptOnly.report, ordinary.report);
process.stdout.write(
	'Installed load receipt: all five paths, exact project bytes/hash/single source receipt, delete-safe codec and informational-only execution passed.\n',
);
