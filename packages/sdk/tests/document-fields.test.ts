// The real addon owns document acquisition, routing and the exact document codec.
import assert from 'node:assert/strict';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

import type { Configuration, DocumentCandidates, SdkDocumentValue } from '../src/index.ts';
import { check, deserialize, inspect, run, serialize } from '../src/index.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

function configuration(base: string, paths: DocumentCandidates): Configuration {
	return {
		id: 'document-coverage',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['docs/requirements.md'] },
		decision_model: { model: 'jev-latest' },
		evidence: {
			kind: 'inline',
			matchers: {
				requirement: { language: 'Markdown', rule: { kind: 'paragraph', regex: '^U[0-9]+:' } },
				document: { language: 'Markdown', rule: { kind: 'document' } },
			},
			kinds: {
				requirement: {
					rule: 'requirement',
					fields: {
						identifier: { from: 'capture', regex: '^(U[0-9]+):' },
						coverage: { from: 'document', path: paths, rule: 'document' },
					},
				},
			},
		},
		rulebooks: [
			{
				kind: 'inline',
				id: 'coverage',
				markdown: 'Requirements name their verification. <!-- K001 -->\n',
				entries: {
					K001: {
						name: 'coverage',
						evidence: 'requirement',
						severity: 'warn',
						decision: {
							start: 'available',
							steps: {
								available: {
									mechanical_check: { tool: 'complete', field: 'coverage' },
									yes: 'covered',
									no: 'not_applicable',
								},
								covered: {
									mechanical_check: {
										tool: 'contains_field',
										field: 'coverage',
										needle: 'identifier',
									},
									yes: 'pass',
									no: { finding: 'Missing coverage.' },
								},
							},
						},
					},
				},
			},
		],
	};
}

async function roundTrip(value: SdkDocumentValue): Promise<void> {
	const saved = serialize(value);
	assert.equal(saved.kind, 'serialized');
	if (saved.kind !== 'serialized') throw new Error('serialization failed');
	const restored = await deserialize(saved.json);
	assert.equal(restored.kind, 'decoded');
	if (restored.kind !== 'decoded') throw new Error('codec rejected document fields');
	assert.deepEqual(restored.value, value);
	assert.ok(Object.isFrozen(restored.value));
}

await test('missing and selected documents survive inspect, run and exact codec round trips', async () => {
	const repository = ownedDirectory('document-fields');
	try {
		writeFiles(repository.path, { 'docs/requirements.md': 'U1: Cover this requirement.\n' });
		const config = configuration(repository.path, ['verification.md', 'verification-design.md']);
		const options = { root: repository.path, scope: { kind: 'all' as const } };
		assert.equal((await check(config)).kind, 'valid');
		const missing = await inspect(config, options);
		assert.equal(missing.kind, 'inspected');
		if (missing.kind !== 'inspected') throw new Error('missing inspection failed');
		assert.deepEqual(missing.report.pieces[0]?.fields['coverage'], {
			state: 'missing_document',
			candidates: ['docs/verification.md', 'docs/verification-design.md'],
		});
		await roundTrip({ kind: 'inspection_outcome', value: missing });
		const skipped = await run(config, options);
		assert.equal(skipped.kind, 'completed');
		if (skipped.kind !== 'completed') throw new Error('missing document did not route');
		assert.equal(skipped.report.results[0]?.outcome, 'not_applicable');
		assert.equal(skipped.report.usage.model_requests, 0);
		await roundTrip({ kind: 'run_outcome', value: skipped });

		const text = '猫 U1 is covered.\n';
		writeFiles(repository.path, { 'docs/verification-design.md': text });
		const selected = await inspect(config, options);
		assert.equal(selected.kind, 'inspected');
		if (selected.kind !== 'inspected') throw new Error('fallback inspection failed');
		const field = selected.report.pieces[0]?.fields['coverage'];
		assert.equal(field?.state, 'document');
		if (field?.state !== 'document') throw new Error('chosen document not recorded');
		assert.equal(field.path, 'docs/verification-design.md');
		assert.deepEqual(field.values, [text]);
		assert.equal(field.spans[0]?.byte_start, 0n);
		assert.equal(field.spans[0]?.byte_end, BigInt(Buffer.byteLength(text)));
		await roundTrip({ kind: 'inspection_report', value: selected.report });
		const covered = await run(config, options);
		assert.equal(covered.kind, 'completed');
		if (covered.kind !== 'completed') throw new Error('fallback run failed');
		assert.equal(covered.report.results[0]?.outcome, 'pass');
		await roundTrip({ kind: 'report', value: covered.report });

		writeFiles(repository.path, { 'docs/verification.md': 'U20 only.\n' });
		const scalar = await inspect(configuration(repository.path, 'verification.md'), options);
		assert.equal(scalar.kind, 'inspected');
		if (scalar.kind !== 'inspected') throw new Error('scalar inspection failed');
		assert.equal(scalar.report.pieces[0]?.fields['coverage']?.state, 'texts');
		const first = await inspect(config, options);
		assert.equal(first.kind, 'inspected');
		if (first.kind !== 'inspected') throw new Error('first candidate inspection failed');
		const firstField = first.report.pieces[0]?.fields['coverage'];
		assert.equal(firstField?.state, 'document');
		if (firstField?.state !== 'document') throw new Error('first candidate not recorded');
		assert.equal(firstField.path, 'docs/verification.md');
		const uncovered = await run(config, options);
		assert.equal(uncovered.kind, 'completed');
		if (uncovered.kind !== 'completed') throw new Error('first candidate run failed');
		assert.equal(uncovered.report.results[0]?.outcome, 'finding');
		assert.equal(uncovered.report.counts.warn, 1);
		writeFileSync(join(repository.path, 'docs/verification.md'), Buffer.from([255]));
		const unreadable = await run(config, options);
		assert.equal(unreadable.kind, 'incomplete', 'unreadable first path must not fall back');
		await roundTrip({ kind: 'run_outcome', value: unreadable });
	} finally {
		repository.remove();
	}
});

await test('document candidate configuration round trips preserve order and reject empty lists', async () => {
	const repository = ownedDirectory('document-config-codec');
	try {
		await Promise.all(
			['verification.md', ['verification.md', 'verification-design.md']].map(async (paths) => {
				const saved = serialize({
					kind: 'configuration',
					value: configuration(repository.path, paths),
				});
				assert.equal(saved.kind, 'serialized');
				if (saved.kind !== 'serialized') throw new Error('configuration not serialized');
				const restored = await deserialize(saved.json);
				assert.equal(restored.kind, 'decoded');
				if (restored.kind !== 'decoded' || restored.value.kind !== 'configuration')
					throw new Error('configuration not decoded');
				const evidence = restored.value.value.evidence;
				assert.equal(evidence.kind, 'inline');
				if (evidence.kind !== 'inline') throw new Error('wrong evidence shape');
				const field = evidence.kinds['requirement']?.fields?.['coverage'];
				assert.equal(field?.from, 'document');
				if (field?.from !== 'document') throw new Error('document field lost');
				assert.deepEqual(field.path, paths);
			}),
		);
		await Promise.all(
			[[], ['']].map(async (paths) => {
				const saved = serialize({
					kind: 'configuration',
					value: configuration(repository.path, paths),
				});
				assert.equal(saved.kind, 'serialized');
				if (saved.kind !== 'serialized') throw new Error('configuration not serialized');
				const restored = await deserialize(saved.json);
				assert.equal(restored.kind, 'invalid');
				if (restored.kind !== 'invalid') throw new Error('invalid candidates accepted');
				assert.equal(restored.reason, 'invalid_value');
				assert.ok(restored.diagnostics.length > 0);
			}),
		);
	} finally {
		repository.remove();
	}
});
