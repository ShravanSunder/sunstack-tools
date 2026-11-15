// Fresh packed consumer: every SDK import resolves from installed bytes.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import type { CaseBinding, CaseSelectionReport, Configuration } from '@sunstack/sdk';
import { contractInfo, deserialize, run, serialize } from '@sunstack/sdk';

const root = join(process.cwd(), 'case-dispatch-checked');
mkdirSync(root);
const firstLine = 'Selected café paragraph.\n';
writeFileSync(join(root, 'one.md'), `# Guide\n\n${firstLine}\nExcluded β paragraph.\n`);
const configuration = {
	id: 'installed-case-dispatch',
	base_directory: root,
	marker_prefix: 'K',
	check: { files: ['*.md'] },
	decision_model: { model: 'jev-latest' },
	evidence: {
		kind: 'inline',
		utilities: {},
		matchers: {},
		kinds: {
			paragraph: {
				markdown_units: { mode: 'unit', units: ['paragraph'] },
				fields: { body: { from: 'remainder' } },
			},
		},
	},
	rulebooks: [
		{
			kind: 'inline',
			id: 'case-rules',
			markdown: 'Each paragraph has a body. <!-- K001 -->\n',
			entries: {
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
								no: { finding: 'Empty.' },
							},
						},
					},
				},
			},
		},
	],
} satisfies Configuration;
const binding = {
	id: 'selected',
	kind: 'rule',
	rule: 'body',
	file: 'one.md',
	lines: [3n, 3n],
	text_sha256: createHash('sha256').update(firstLine).digest('hex'),
} as const satisfies CaseBinding;
const contract = await contractInfo();
assert.equal(contract.kind, 'ready');
if (contract.kind !== 'ready') throw new Error('installed contract unavailable');
assert.equal(contract.contract.sdk_release, '0.23.0');
assert.equal(contract.contract.contract_revision, 20);
const options = { root, scope: { kind: 'all' } } as const;
const ordinary = await run(configuration, options);
assert.equal(ordinary.kind, 'completed');
if (ordinary.kind !== 'completed') throw new Error('ordinary control failed');
assert.equal(ordinary.report.results.length, 2);
assert.equal(ordinary.report.case_selection, undefined);
const selected = await run(configuration, {
	...options,
	case_selection: [binding, { ...binding, id: 'duplicate-owner' }],
});
assert.equal(selected.kind, 'completed');
if (selected.kind !== 'completed') throw new Error('selected run failed');
assert.equal(selected.report.results.length, 1);
assert.equal(selected.report.results[0]?.first_line, 3n);
const accounting: CaseSelectionReport | undefined | null = selected.report.case_selection;
assert.ok(accounting !== undefined && accounting !== null);
assert.deepEqual(
	accounting.bindings.map((result) => result.binding.id),
	['selected', 'duplicate-owner'],
);
assert.ok(accounting.bindings.every((result) => result.outcome.kind === 'selected'));
assert.equal(accounting.excluded.length, 1);
const saved = serialize({ kind: 'run_outcome', value: selected });
assert.equal(saved.kind, 'serialized');
if (saved.kind !== 'serialized') throw new Error('report serialization failed');
const restored = await deserialize(saved.json);
assert.equal(restored.kind, 'decoded');
if (restored.kind !== 'decoded') throw new Error('installed codec failed');
assert.deepEqual(restored.value, { kind: 'run_outcome', value: selected });
const incompatible = await deserialize(
	saved.json.replace('"contract_revision":20', '"contract_revision":19'),
);
assert.equal(incompatible.kind, 'incompatible');
const empty = await run(configuration, { ...options, case_selection: [] });
assert.equal(empty.kind, 'completed');
if (empty.kind !== 'completed') throw new Error('empty selection failed');
assert.equal(empty.report.results.length, 0);
assert.deepEqual(empty.report.case_selection?.bindings, []);
process.stdout.write(
	'Installed case selection: native selected/ordinary/empty controls, typed accounting, exact codec and revision refusal passed.\n',
);
