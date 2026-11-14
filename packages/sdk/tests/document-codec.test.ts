// Real-addon codec coverage: typed roots, exact raw tokens and explicit refusals.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { inspect as formatValue } from 'node:util';

import type { Configuration, DeserializationOutcome, SdkDocumentValue } from '../src/index.ts';
import { check, contractInfo, deserialize, inspect, load, run, serialize } from '../src/index.ts';
import {
	inlineConfiguration,
	ownedDirectory,
	writeFileProject,
	writeFiles,
} from './support/sdk-fixtures.ts';

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

const fixture = ownedDirectory('document-codec');
writeFileProject(fixture.path, 'http://127.0.0.1:1/v1/systemone');
writeFiles(fixture.path, { 'docs/one.md': '# One\n\nA paragraph.\n' });

const configuration: Configuration = {
	id: 'codec',
	base_directory: fixture.path,
	marker_prefix: 'K',
	check: { files: ['docs/*.md'] },
	decision_model: { model: 'jev-latest' },
	limits: { max_file_bytes: 42n },
	evidence: {
		kind: 'inline',
		utilities: {},
		matchers: { paragraph: { language: 'Markdown', rule: { kind: 'paragraph' } } },
		kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
	},
	rulebooks: [
		{
			kind: 'inline',
			id: 'codec-rules',
			markdown: 'Each paragraph has a body. <!-- K001 -->\n',
			entries: {
				K001: {
					name: 'body',
					evidence: 'paragraph',
					severity: 'error',
					reference: { codec_boundary: '__CODEC_TOKEN__' },
					decision: {
						start: 'has',
						steps: {
							has: {
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
};

function saved(value: SdkDocumentValue): string {
	const outcome = serialize(value);
	assert.equal(outcome.kind, 'serialized');
	return outcome.json;
}

async function decoded(json: string): Promise<SdkDocumentValue> {
	const outcome = await deserialize(json);
	assert.equal(outcome.kind, 'decoded', formatValue(outcome, { depth: null }));
	if (outcome.kind !== 'decoded') throw new Error('document was not decoded');
	assert.ok(Object.isFrozen(outcome));
	assert.ok(Object.isFrozen(outcome.value));
	assert.ok(Object.isFrozen(outcome.value.value));
	return outcome.value;
}

function rawReference(token: string): string {
	const json = saved({ kind: 'configuration', value: configuration });
	assert.ok(json.includes('"__CODEC_TOKEN__"'));
	return json.replace('"__CODEC_TOKEN__"', token);
}

async function invalid(json: string, reason: string): Promise<DeserializationOutcome> {
	const outcome = await deserialize(json);
	assert.equal(outcome.kind, 'invalid');
	assert.equal(outcome.reason, reason);
	assert.ok(outcome.diagnostics.length > 0);
	return outcome;
}

/** Every authored field survives; Rust may add defaults to objects along the way. */
function assertAuthoredFields(actual: unknown, authored: unknown, path: string): void {
	if (Array.isArray(authored)) {
		assert.ok(Array.isArray(actual), path);
		assert.equal(actual.length, authored.length, path);
		authored.forEach((item: unknown, index) =>
			assertAuthoredFields(actual[index], item, `${path}[${index}]`),
		);
		return;
	}
	if (typeof authored === 'object' && authored !== null) {
		assert.ok(typeof actual === 'object' && actual !== null && !Array.isArray(actual), path);
		for (const [key, value] of Object.entries(authored)) {
			assert.ok(Object.hasOwn(actual, key), `${path}.${key}`);
			assertAuthoredFields(Reflect.get(actual, key), value, `${path}.${key}`);
		}
		return;
	}
	assert.deepEqual(actual, authored, path);
}

await test('all eight real SDK roots survive a typed round trip and normalization', async () => {
	const loaded = await load({ directory: fixture.path });
	assert.equal(loaded.kind, 'loaded');
	if (loaded.kind !== 'loaded') throw new Error('fixture failed to load');
	const checked = await check(configuration);
	assert.equal(checked.kind, 'valid');
	const options = { root: fixture.path, scope: { kind: 'all' as const } };
	const ran = await run(configuration, options);
	assert.equal(ran.kind, 'completed');
	if (ran.kind !== 'completed') throw new Error('fixture run did not complete');
	const inspected = await inspect(configuration, options);
	assert.equal(inspected.kind, 'inspected');
	if (inspected.kind !== 'inspected') throw new Error('fixture inspection did not complete');
	const reportDocument = saved({ kind: 'run_outcome', value: ran });
	assert.ok(
		reportDocument.includes('"piece_evidence":{"1":'),
		'the real run exercised a numbered map',
	);
	await Promise.all(
		['bad', '-1', '18446744073709551616', ' 1 '].map((key) =>
			invalid(
				reportDocument.replace('"piece_evidence":{"1":', `"piece_evidence":{"${key}":`),
				'invalid_value',
			),
		),
	);
	const roots: readonly SdkDocumentValue[] = [
		{ kind: 'configuration', value: configuration },
		{ kind: 'loaded_configuration', value: loaded.value },
		{ kind: 'load_outcome', value: loaded },
		{ kind: 'check_outcome', value: checked },
		{ kind: 'run_outcome', value: ran },
		{ kind: 'inspection_outcome', value: inspected },
		{ kind: 'report', value: ran.report },
		{ kind: 'inspection_report', value: inspected.report },
	];
	await Promise.all(
		roots.map(async (root) => {
			const first = await decoded(saved(root));
			assert.equal(first.kind, root.kind);
			if (root.kind === 'configuration' && first.kind === 'configuration') {
				assertAuthoredFields(first.value, root.value, 'configuration');
			} else {
				assert.deepEqual(first, root, `${root.kind} first read`);
			}
			// The first read may fill serde defaults. Its second read must be identical.
			assert.deepEqual(await decoded(saved(first)), first, root.kind);
		}),
	);
	const restored = await decoded(
		saved(roots[0] ?? { kind: 'configuration', value: configuration }),
	);
	assert.equal(restored.kind, 'configuration');
	if (restored.kind === 'configuration') {
		assert.equal(restored.value.limits?.max_file_bytes, 42n);
		assert.deepEqual(restored.value.check.ignore, []);
	}
});

await test('independent numeric tokens retain integers, strings and floats', async () => {
	const restored = await decoded(
		rawReference(
			'{"minimum":-9223372036854775808,"maximum":18446744073709551615,"small":3,"fraction":0.5,"text":"18446744073709551615"}',
		),
	);
	assert.equal(restored.kind, 'configuration');
	if (restored.kind !== 'configuration') throw new Error('wrong root');
	const rulebook = restored.value.rulebooks[0];
	assert.equal(rulebook?.kind, 'inline');
	if (rulebook?.kind !== 'inline') throw new Error('wrong rulebook');
	assert.deepEqual(rulebook.entries['K001']?.reference?.['codec_boundary'], {
		minimum: -9223372036854775808n,
		maximum: 18446744073709551615n,
		small: 3n,
		fraction: 0.5,
		text: '18446744073709551615',
	});
});

await test('captured generic entries widen their bound structured integer type', async () => {
	const loaded = await load({ directory: fixture.path });
	assert.equal(loaded.kind, 'loaded');
	if (loaded.kind !== 'loaded') throw new Error('fixture failed to load');
	const rulebook = loaded.value.configuration.rulebooks[0];
	if (rulebook?.kind !== 'captured') throw new Error('expected captured rulebook');
	const entry = rulebook.entries.value['K001'];
	if (entry === undefined) throw new Error('missing captured entry');
	const edited = {
		...loaded.value,
		configuration: {
			...loaded.value.configuration,
			rulebooks: [
				{
					...rulebook,
					entries: {
						...rulebook.entries,
						value: {
							...rulebook.entries.value,
							K001: { ...entry, reference: { codec_boundary: 5n } },
						},
					},
				},
			],
		},
	};
	const restored = await decoded(saved({ kind: 'loaded_configuration', value: edited }));
	assert.equal(restored.kind, 'loaded_configuration');
	if (restored.kind !== 'loaded_configuration') throw new Error('wrong root');
	const restoredRulebook = restored.value.configuration.rulebooks[0];
	if (restoredRulebook?.kind !== 'captured') throw new Error('expected captured rulebook');
	assert.equal(restoredRulebook.entries.value['K001']?.reference?.['codec_boundary'], 5n);
});

await test('overflow, unsafe integral floats and typed f64 rounding are refused with paths', async () => {
	await Promise.all(
		['18446744073709551616', '-9223372036854775809', '9007199254740993.0', '1e400'].map(
			async (token) => {
				const outcome = await invalid(rawReference(token), 'unsupported_number');
				if (outcome.kind === 'invalid')
					assert.equal(outcome.diagnostics[0]?.json_path.at(-1), 'codec_boundary');
			},
		),
	);
	const asking = saved({
		kind: 'configuration',
		value: inlineConfiguration(fixture.path, 'http://127.0.0.1:1'),
	});
	assert.ok(asking.includes('"yes_at_or_above":0.8'));
	const rounded = await invalid(
		asking.replace('"yes_at_or_above":0.8', '"yes_at_or_above":9007199254740993'),
		'unsupported_number',
	);
	if (rounded.kind === 'invalid')
		assert.equal(rounded.diagnostics[0]?.json_path.at(-1), 'yes_at_or_above');
});

await test('syntax, wrapper, root, duplicates and revision have distinct endings', async () => {
	const contract = await contractInfo();
	assert.equal(contract.kind, 'ready');
	if (contract.kind !== 'ready') throw new Error('addon unavailable');
	const revision = contract.contract.contract_revision;
	await invalid('{', 'malformed_json');
	await invalid(`{"contract_revision":${revision},"payload":null,"extra":1}`, 'invalid_document');
	await invalid(
		`{"contract_revision":${revision},"payload":{"kind":"unknown","value":{}}}`,
		'invalid_value',
	);
	const document = rawReference('1');
	await invalid(document.replace('"id":"codec"', '"id":"codec","id":"duplicate"'), 'invalid_value');
	const incompatible = await deserialize(
		`{"contract_revision":${revision + 1},"payload":{"future":184467440737095516160000}}`,
	);
	assert.equal(incompatible.kind, 'incompatible');
	if (incompatible.kind === 'incompatible')
		assert.equal(incompatible.actual_contract_revision, revision + 1);
});

await test('Unicode remains exact or is refused; writing refuses unsupported data without mutation', async () => {
	const restored = await decoded(rawReference('"😀 café"'));
	assert.equal(restored.kind, 'configuration');
	if (restored.kind !== 'configuration') throw new Error('wrong root');
	const rulebook = restored.value.rulebooks[0];
	if (rulebook?.kind !== 'inline') throw new Error('wrong rulebook');
	assert.equal(rulebook.entries['K001']?.reference?.['codec_boundary'], '😀 café');
	assert.notEqual((await deserialize(rawReference('"\\ud800"'))).kind, 'decoded');
	assert.notEqual((await deserialize('\ud800')).kind, 'decoded');
	const before = structuredClone(configuration);
	assert.equal(serialize({ kind: 'configuration', value: configuration }).kind, 'serialized');
	assert.deepEqual(configuration, before);
	const overflow = structuredClone(configuration);
	const first = overflow.rulebooks[0];
	if (first?.kind !== 'inline') throw new Error('wrong rulebook');
	const entry = first.entries['K001'];
	if (entry === undefined) throw new Error('missing entry');
	const invalidValue = {
		...overflow,
		rulebooks: [
			{ ...first, entries: { K001: { ...entry, reference: { codec_boundary: 2n ** 64n } } } },
		],
	};
	assert.equal(serialize({ kind: 'configuration', value: invalidValue }).kind, 'rejected');
	const dynamic: unknown = Reflect.apply(serialize, undefined, [
		{ kind: 'configuration', value: { callback: () => {} } },
	]);
	assert.deepEqual(
		typeof dynamic === 'object' && dynamic !== null ? Reflect.get(dynamic, 'kind') : null,
		'rejected',
	);
	assert.deepEqual(configuration, before);
});

fixture.remove();
