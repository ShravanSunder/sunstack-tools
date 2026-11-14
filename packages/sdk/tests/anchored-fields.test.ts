import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Configuration, FieldValue } from '../src/index.ts';
import { check, deserialize, inspect, serialize } from '../src/index.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

await test('anchored kind fields and scoped row lists cross the real addon and codec', async () => {
	const repository = ownedDirectory('anchored-fields');
	try {
		writeFiles(repository.path, {
			'docs/requirements.html': "<p id='U1'>First</p>\n<p id='U2'>Second</p>",
			'docs/specification.html':
				"<article id='R1'><a href='requirements.html#U1'>needs</a><i>U1</i></article>\n<article id='R2'><a href='requirements.html#U2'>needs</a><i>U2</i><i>U4</i></article>",
			'docs/trace.html': '<p>U1 R1 trace</p>\n<p>U2 R2 trace</p>\n<p>U20 unrelated</p>',
			'docs/verification.html':
				'<p>U1 R1 proof</p>\n<p>U2 R2 proof</p>\n<p>U4 (R1–R3, R5, R7)</p>\n<p>U20 unrelated</p>',
		});
		const config: Configuration = {
			id: 'anchored-fields',
			base_directory: repository.path,
			marker_prefix: 'K',
			check: { files: ['docs/requirements.html', 'docs/specification.html'] },
			decision_model: { model: 'jev-latest' },
			evidence: {
				kind: 'inline',
				utilities: {},
				matchers: {
					requirement: { language: 'Html', rule: { kind: 'element', regex: "^<p id='U" } },
					obligation: { language: 'Html', rule: { kind: 'element', regex: '^<article' } },
					identifier: { language: 'Html', rule: { kind: 'text', regex: '^U[0-9]+$' } },
					row: { language: 'Html', rule: { kind: 'element', regex: '^<p' } },
				},
				kinds: {
					requirement_text: { rule: 'requirement', fields: { body: { from: 'remainder' } } },
					requirement: {
						rule: 'requirement',
						fields: {
							identifier: { from: 'capture', regex: "id='([^']+)'" },
							obligations: {
								from: 'document',
								path: ['missing.html', 'specification.html'],
								rule: 'obligation',
								containing: 'identifier',
							},
							trace: {
								from: 'document',
								path: 'trace.html',
								rule: 'row',
								containing: 'identifier',
							},
							verification: {
								from: 'document',
								path: 'verification.html',
								rule: 'row',
								containing: 'identifier',
							},
						},
					},
					obligation: {
						rule: 'obligation',
						fields: {
							needs: {
								from: 'target',
								regex: "href='([^']+)'",
								resolve: 'path',
								as_kind: 'requirement_text',
							},
							requirement_ids: { from: 'inside', rule: 'identifier' },
							verification: {
								from: 'document',
								path: 'verification.html',
								rule: 'row',
								containing: 'requirement_ids',
							},
						},
					},
				},
			},
			rulebooks: [
				{
					kind: 'inline',
					id: 'coverage',
					markdown: 'Coverage is available.<!-- K001 -->\n',
					entries: {
						K001: {
							name: 'coverage',
							evidence: 'requirement',
							decision: {
								start: 'complete',
								steps: {
									complete: {
										mechanical_check: { tool: 'complete', field: 'trace' },
										yes: 'pass',
										no: { finding: 'Missing trace.' },
									},
								},
							},
						},
					},
				},
			],
		};
		assert.equal((await check(config)).kind, 'valid');
		const inspected = await inspect(config, { root: repository.path, scope: { kind: 'all' } });
		assert.equal(inspected.kind, 'inspected');
		if (inspected.kind !== 'inspected') throw new Error('inspection did not complete');
		const secondRequirement = inspected.report.pieces.find(
			(piece) => piece.kind === 'requirement' && piece.text.includes("id='U2'"),
		);
		assert.ok(secondRequirement !== undefined);
		for (const name of ['obligations', 'trace', 'verification']) {
			const field: FieldValue | undefined = secondRequirement.fields[name];
			assert.ok(field?.state === 'texts' || field?.state === 'document');
			assert.equal(field.values.length, 1);
			assert.match(field.values[0] ?? '', /U2/u);
			assert.doesNotMatch(field.values[0] ?? '', /U20/u);
		}
		const secondObligation = inspected.report.pieces.find(
			(piece) => piece.kind === 'obligation' && piece.text.includes("id='R2'"),
		);
		assert.ok(secondObligation !== undefined);
		const needs = secondObligation.fields['needs'];
		assert.equal(needs?.state, 'targets');
		if (needs?.state !== 'targets') throw new Error('target unavailable');
		assert.deepEqual(needs.values[0]?.anchor, { status: 'found', fragment: 'U2' });
		const body = needs.values[0]?.fields?.['body'];
		assert.equal(body?.state, 'text');
		if (body?.state !== 'text') throw new Error('anchored body unavailable');
		assert.equal(body.value, "<p id='U2'>Second</p>");
		const verification = secondObligation.fields['verification'];
		assert.equal(verification?.state, 'texts');
		if (verification?.state !== 'texts') throw new Error('list filter unavailable');
		assert.deepEqual(verification.values, ['<p>U2 R2 proof</p>', '<p>U4 (R1–R3, R5, R7)</p>']);
		const saved = serialize({ kind: 'inspection_outcome', value: inspected });
		assert.equal(saved.kind, 'serialized');
		if (saved.kind !== 'serialized') throw new Error('serialization failed');
		const restored = await deserialize(saved.json);
		assert.equal(restored.kind, 'decoded');
		if (restored.kind !== 'decoded') throw new Error('round trip failed');
		assert.deepEqual(restored.value, { kind: 'inspection_outcome', value: inspected });
	} finally {
		repository.remove();
	}
});
