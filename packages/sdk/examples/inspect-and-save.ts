// Inspect static TypeScript call sites, save the typed outcome, and restore it.
// node inspect-and-save.ts <absolute-checked-root> <output-file>
// Expected input: src/**/*.ts with at least one console.log(...) call.
import { readFile, writeFile } from 'node:fs/promises';
import { inspect as formatValue } from 'node:util';

import { deserialize, inspect, serialize } from '@sunstack/sdk';
import type { Configuration } from '@sunstack/sdk';

const [root = process.cwd(), destination = 'inspection.sdk.json'] = process.argv.slice(2);
const configuration = {
	id: 'call-sites',
	base_directory: import.meta.dirname,
	marker_prefix: 'K',
	check: { files: ['src/**/*.ts'] },
	decision_model: { model: 'jev-latest' },
	evidence: {
		kind: 'inline',
		utilities: {},
		matchers: { calls: { language: 'TypeScript', rule: { pattern: 'console.log($$$ARGS)' } } },
		kinds: { call: { rule: 'calls', fields: { body: { from: 'first_line' } } } },
	},
	rulebooks: [],
} satisfies Configuration;

// Inspection admits its own definitions, permitting no rulebooks. Ordinary check/run
// still require rulebooks. Static kinds use no credential and evaluate no rules.
const outcome = await inspect(configuration, { root, scope: { kind: 'all' } });
if (outcome.kind !== 'inspected') {
	process.stderr.write(`${formatValue(outcome, { depth: null })}\n`);
	process.exit(1);
}
const { report } = outcome;
process.stdout.write(`inspection: ${report.status.kind}; rules were not evaluated\n`);
process.stdout.write(
	`coverage: ${report.file_counts.length} file(s), ${report.pieces.length} piece(s)\n`,
);
if (
	report.status.kind !== 'clean' ||
	report.file_counts.length === 0 ||
	report.pieces.length === 0
) {
	process.stderr.write(
		'This recipe expects a complete listing with checked files and call sites.\n',
	);
	process.exit(1);
}
for (const piece of report.pieces) {
	const { span } = piece.where;
	process.stdout.write(
		`${piece.where.file}:${span.start_line} bytes ${span.byte_start}-${span.byte_end}: ${piece.text}\n`,
	);
}

const saved = serialize({ kind: 'inspection_outcome', value: outcome });
if (saved.kind !== 'serialized') {
	process.stderr.write(`${saved.reason}: ${saved.message}\n`);
	process.exit(1);
}
// File paths, atomicity and retention belong to this application, not the SDK.
await writeFile(destination, saved.json, 'utf8');
const restored = await deserialize(await readFile(destination, 'utf8'));
if (restored.kind !== 'decoded' || restored.value.kind !== 'inspection_outcome') {
	process.stderr.write(`${formatValue(restored, { depth: null })}\n`);
	process.exit(1);
}
// A decoded outcome is data, not a new inspection or authenticated producer proof.
const readback = serialize(restored.value);
if (readback.kind !== 'serialized' || readback.json !== saved.json) {
	process.stderr.write('Readback did not retain this inspection outcome.\n');
	process.exit(1);
}
process.stdout.write(`restored exact typed outcome from ${destination}\n`);

// The SDK owns the real header; this deliberately incompatible fixture demonstrates
// refusal before its payload is interpreted. It is not a migration recipe.
const future = saved.json.replace(/"contract_revision":\d+/u, '"contract_revision":4294967295');
const refused = await deserialize(future);
if (refused.kind !== 'incompatible') {
	process.stderr.write(`${formatValue(refused, { depth: null })}\n`);
	process.exit(1);
}
process.stdout.write(
	`revision refused: expected ${refused.expected_contract_revision}, got ${refused.actual_contract_revision}\n`,
);
