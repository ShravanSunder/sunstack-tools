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
assert.equal(contract.contract.sdk_release, '0.24.0');
assert.equal(contract.contract.contract_revision, 23);
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
	saved.json.replace('"contract_revision":23', '"contract_revision":19'),
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

// Same-line ast-grep pieces and byte identity through the normal packed addon.
writeFileSync(join(root, 'page.html'), '<p>café</p><p>βeta</p>\r\n');
const byteConfiguration = {
	...configuration,
	check: { files: ['*.html'] },
	evidence: {
		kind: 'inline',
		utilities: {},
		matchers: { paragraph: { language: 'Html', rule: { kind: 'element', regex: '^<p>' } } },
		kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
	},
} satisfies Configuration;
const byteBinding = {
	...binding,
	id: 'bytes-first',
	file: 'page.html',
	lines: [1n, 1n],
	bytes: [0n, 12n],
	text_sha256: createHash('sha256').update('<p>café</p>').digest('hex'),
} as const satisfies CaseBinding;
const byteRun = await run(byteConfiguration, {
	...options,
	case_selection: [byteBinding, { ...byteBinding, id: 'bytes-same-owner' }],
});
assert.equal(byteRun.kind, 'completed');
if (byteRun.kind !== 'completed') throw new Error('installed byte selection failed');
assert.equal(byteRun.report.results.length, 1);
assert.deepEqual(
	byteRun.report.case_selection?.bindings.map((item) => item.outcome),
	[
		{
			kind: 'selected',
			owner: { kind: 'rule', rule: 'body', file: 'page.html', lines: [1n, 1n], bytes: [0n, 12n] },
		},
		{
			kind: 'selected',
			owner: { kind: 'rule', rule: 'body', file: 'page.html', lines: [1n, 1n], bytes: [0n, 12n] },
		},
	],
);
assert.deepEqual(byteRun.report.case_selection?.excluded, [
	{ kind: 'rule', rule: 'body', file: 'page.html', lines: [1n, 1n], bytes: [12n, 24n] },
]);
const wideBinding = {
	...byteBinding,
	id: 'bytes-wide',
	file: 'outside/page.html',
	bytes: [9007199254740993n, 9007199254740994n],
} as const satisfies CaseBinding;
const wideRun = await run(byteConfiguration, { ...options, case_selection: [wideBinding] });
assert.equal(wideRun.kind, 'completed');
if (wideRun.kind !== 'completed') throw new Error('installed full-width transport failed');
assert.deepEqual(wideRun.report.case_selection?.bindings, [
	{ binding: wideBinding, outcome: { kind: 'unmatched' } },
]);
assert.equal(wideRun.report.results.length, 0);
await Promise.all(
	[byteRun, wideRun].map(async (outcome): Promise<void> => {
		const document = serialize({ kind: 'run_outcome', value: outcome });
		assert.equal(document.kind, 'serialized');
		if (document.kind !== 'serialized')
			throw new Error('installed byte report serialization failed');
		const readback = await deserialize(document.json);
		assert.equal(readback.kind, 'decoded');
		if (readback.kind !== 'decoded') throw new Error('installed byte codec failed');
		assert.deepEqual(readback.value, { kind: 'run_outcome', value: outcome });
	}),
);
process.stdout.write(
	'Installed byte selection: same-line exact owners, execution deduplication and full-width unacquired tuple codec passed.\n',
);
