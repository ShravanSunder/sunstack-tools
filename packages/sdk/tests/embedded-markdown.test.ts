// Released 0.18 settings through the real native boundary and exact document codec.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { test } from 'node:test';
import { inspect as formatValue } from 'node:util';

import { CONTRACT_REVISION } from '../src/contracts/contract-revision.ts';
import type {
	Configuration,
	ConfigurationInput,
	HtmlEvidenceSettings,
	InspectionReport,
	SdkDocumentValue,
} from '../src/index.ts';
import { check, contractInfo, deserialize, inspect, load, run, serialize } from '../src/index.ts';
import { isCheckOutcome } from '../src/reply-decoding.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

const page =
	'<p>Before</p>猫<script type="text/markdown">## First\n\nOne café.\n\nTwo.\n</script><p>After</p><script type="text/markdown">Three.\n</script><script type="other">Decoy.\n</script>';
const selected = {
	embedded_markdown: [{ tag: 'script', attributes: { type: 'text/markdown' } }],
} as const satisfies HtmlEvidenceSettings;
const absent = {
	embedded_markdown: [{ tag: 'custom-source' }],
} as const satisfies HtmlEvidenceSettings;
const kinds = {
	html: { rule: 'html', fields: { body: { from: 'remainder' } } },
	paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } },
} as const;
const entries = {
	K001: {
		name: 'body',
		evidence: 'paragraph',
		severity: 'error',
		decision: {
			start: 'complete',
			steps: {
				complete: {
					mechanical_check: { tool: 'complete', field: 'body' },
					yes: 'pass',
					no: { finding: 'Missing body.' },
				},
			},
		},
	},
} as const;

function configuration(base: string, html?: HtmlEvidenceSettings): Configuration {
	return {
		id: 'embedded-markdown',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['docs/*.html'] },
		decision_model: { model: 'jev-latest' },
		evidence: {
			kind: 'inline',
			utilities: {},
			kinds,
			matchers: {
				html: { language: 'Html', rule: { kind: 'element', regex: '^<p>' } },
				paragraph: { language: 'Markdown', rule: { kind: 'paragraph' } },
			},
			...(html === undefined ? {} : { html }),
		},
		rulebooks: [
			{ kind: 'inline', id: 'rules', markdown: 'Body is complete. <!-- K001 -->\n', entries },
		],
	};
}

async function inspected(input: ConfigurationInput, root: string): Promise<InspectionReport> {
	const result = await inspect(input, { root, scope: { kind: 'all' } });
	assert.equal(result.kind, 'inspected', formatValue(result, { depth: null }));
	if (result.kind !== 'inspected') throw new Error('inspection did not complete');
	assert.equal(result.report.status.kind, 'clean');
	return result.report;
}

async function roundTrip(value: SdkDocumentValue): Promise<SdkDocumentValue> {
	const saved = serialize(value);
	assert.equal(saved.kind, 'serialized');
	if (saved.kind !== 'serialized') throw new Error('value was not saved');
	const restored = await deserialize(saved.json);
	assert.equal(restored.kind, 'decoded', formatValue(restored, { depth: null }));
	if (restored.kind !== 'decoded') throw new Error('value was not restored');
	return restored.value;
}

await test('configured islands preserve mixed source order, exact UTF-8 spans and run evidence', async () => {
	const repository = ownedDirectory('embedded-markdown');
	try {
		writeFiles(repository.path, { 'docs/page.html': page });
		const config = configuration(repository.path, selected);
		assert.equal((await check(config)).kind, 'valid');
		const report = await inspected(config, repository.path);
		assert.deepEqual(
			report.pieces.map((piece) => [piece.kind, piece.text]),
			[
				['html', '<p>Before</p>'],
				['paragraph', 'One café.\n'],
				['paragraph', 'Two.\n'],
				['html', '<p>After</p>'],
				['paragraph', 'Three.\n'],
			],
		);
		for (const piece of report.pieces) {
			const start = page.indexOf(piece.text);
			const span = piece.where.span;
			assert.equal(span.byte_start, BigInt(Buffer.byteLength(page.slice(0, start))));
			assert.equal(
				span.byte_end,
				BigInt(Buffer.byteLength(page.slice(0, start + piece.text.length))),
			);
			assert.equal(span.source.kind, 'document');
			if (span.source.kind !== 'document') throw new Error('wrong source identity');
			assert.equal(span.source.path, 'docs/page.html');
			assert.equal(span.source.content_identity, createHash('sha256').update(page).digest('hex'));
		}
		const ran = await run(config, { root: repository.path, scope: { kind: 'all' } });
		assert.equal(ran.kind, 'completed');
		if (ran.kind !== 'completed') throw new Error('run did not complete');
		assert.equal(ran.report.status.kind, 'clean');
		assert.equal(ran.report.results.length, 3);
		assert.deepEqual(
			Object.values(ran.report.piece_evidence).map((item) => item.piece.text),
			['One café.\n', 'Two.\n', 'Three.\n'],
		);
		assert.deepEqual(await roundTrip({ kind: 'inspection_report', value: report }), {
			kind: 'inspection_report',
			value: report,
		});
		assert.deepEqual(await roundTrip({ kind: 'run_outcome', value: ran }), {
			kind: 'run_outcome',
			value: ran,
		});
	} finally {
		repository.remove();
	}
});

await test('omitted, empty and nonmatching selectors acquire no Markdown and preserve default serialization', async () => {
	const repository = ownedDirectory('island-defaults');
	try {
		writeFiles(repository.path, { 'docs/page.html': page });
		await Promise.all(
			[undefined, {}, { embedded_markdown: [] }, absent].map(async (html) => {
				const report = await inspected(configuration(repository.path, html), repository.path);
				assert.deepEqual(
					report.pieces.map((piece) => piece.text),
					['<p>Before</p>', '<p>After</p>'],
				);
			}),
		);
		await Promise.all(
			[undefined, {}, { embedded_markdown: [] }].map(async (html) => {
				const restored = await roundTrip({
					kind: 'configuration',
					value: configuration(repository.path, html),
				});
				assert.equal(restored.kind, 'configuration');
				if (restored.kind !== 'configuration') throw new Error('wrong root');
				assert.equal(restored.value.evidence.html, undefined);
			}),
		);
		const contract = await contractInfo();
		assert.equal(contract.kind, 'ready');
		if (contract.kind !== 'ready') throw new Error('addon unavailable');
		assert.equal(contract.contract.sdk_release, '0.20.1');
		assert.equal(contract.contract.contract_revision, 14);
		const previous = await deserialize(
			'{"contract_revision":13,"payload":{"kind":"configuration","value":{}}}',
		);
		assert.equal(previous.kind, 'incompatible');
	} finally {
		repository.remove();
	}
});

await test('loaded settings remain captured after disk edits and typed edits govern restored execution', async () => {
	const repository = ownedDirectory('island-provenance');
	try {
		const rootConfig = (html: HtmlEvidenceSettings): string =>
			JSON.stringify({
				marker_prefix: 'K',
				check: { files: ['docs/*.html'] },
				decision_model: { model: 'jev-latest' },
				rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
				evidence: { ast_grep: 'evidence/sgconfig.yml', kinds, html },
			});
		writeFiles(repository.path, {
			'sunstack.lint.json': rootConfig(selected),
			'docs/page.html': page,
			'rulebook.md': 'Body is complete. <!-- K001 -->\n',
			'rules.json': JSON.stringify(entries),
			'evidence/sgconfig.yml': 'ruleDirs: [rules]\n',
			'evidence/rules/pieces.yml':
				'id: html\nlanguage: Html\nrule: {kind: element, regex: "^<p>"}\n---\nid: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n',
		});
		const loaded = await load({ directory: repository.path });
		assert.equal(loaded.kind, 'loaded');
		if (loaded.kind !== 'loaded') throw new Error('fixture did not load');
		writeFiles(repository.path, { 'sunstack.lint.json': rootConfig(absent) });
		assert.equal((await inspected(loaded.value, repository.path)).pieces.length, 5);
		const restored = await roundTrip({ kind: 'loaded_configuration', value: loaded.value });
		assert.equal(restored.kind, 'loaded_configuration');
		if (restored.kind !== 'loaded_configuration') throw new Error('wrong restored root');
		assert.deepEqual(restored.value, loaded.value);
		assert.equal((await inspected(restored.value, repository.path)).pieces.length, 5);
		const edited: Configuration = {
			...restored.value.configuration,
			evidence: { ...restored.value.configuration.evidence, html: absent },
		};
		assert.equal((await check(edited)).kind, 'valid');
		assert.equal((await inspected(edited, repository.path)).pieces.length, 2);
		const reloaded = await load({ directory: repository.path });
		assert.equal(reloaded.kind, 'loaded');
		if (reloaded.kind !== 'loaded') throw new Error('fixture did not reload');
		assert.equal((await inspected(reloaded.value, repository.path)).pieces.length, 2);
	} finally {
		repository.remove();
	}
});

await test('Rust refuses invalid selector shapes at both check and document decode boundaries', async () => {
	const repository = ownedDirectory('island-refusals');
	try {
		const config = configuration(repository.path);
		const invalidSettings = [
			{ embedded_markdown: [{ tag: 'script', unknown: true }] },
			{ embedded_markdown: [{}] },
			{ embedded_markdown: null },
			{ embedded_markdown: [{ tag: 'script', attributes: null }] },
			{ embedded_markdown: [{ tag: ' ' }] },
			{ embedded_markdown: [{ tag: 'script', attributes: { '': 'x' } }] },
		];
		await Promise.all(
			invalidSettings.map(async (html) => {
				const value = { ...config, evidence: { ...config.evidence, html } };
				const checked: unknown = await Reflect.apply(check, undefined, [value]);
				assert.ok(isCheckOutcome(checked));
				assert.equal(checked.kind, 'invalid');
				const decoded = await deserialize(
					JSON.stringify({
						contract_revision: CONTRACT_REVISION,
						payload: { kind: 'configuration', value },
					}),
				);
				assert.equal(decoded.kind, 'invalid');
				if (decoded.kind !== 'invalid') throw new Error('invalid selector was decoded');
				assert.equal(decoded.reason, 'invalid_value');
			}),
		);
	} finally {
		repository.remove();
	}
});
