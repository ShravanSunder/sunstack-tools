/** Observe independently authored document refusals through the installed Rust-backed reader. */
import path from 'node:path';

import { z } from 'zod';

import { boundedSdkWait, type SdkControlledBackend } from './sdk-controlled-backend.ts';
import { sdkInlineConfiguration } from './sdk-fixtures.ts';
import type { JourneyResult } from './sdk-journeys.ts';
import { requireSdk, saveSdkEvidence, type SdkExports, type SdkTarget } from './sdk-target.ts';

const outcomeSchema = z.object({ kind: z.string() }).passthrough();

export async function observeSdkCodecRefusals(options: {
	readonly sdk: SdkExports;
	readonly target: SdkTarget;
	readonly evidence: string;
	readonly backend: SdkControlledBackend;
}): Promise<JourneyResult> {
	const { sdk, target, evidence, backend } = options;
	requireSdk(sdk.deserialize, 'Installed SDK does not export deserialize');
	requireSdk(sdk.serialize, 'Installed SDK does not export serialize');
	const deserialize = sdk.deserialize;
	const beforeRequests = backend.captures.length;
	const input = z
		.record(z.string(), z.unknown())
		.parse(sdkInlineConfiguration(evidence, backend.endpoint));
	const rulebooks = z.array(z.record(z.string(), z.unknown())).parse(input['rulebooks']);
	const firstRulebook = rulebooks[0];
	requireSdk(firstRulebook, 'Codec fixture has no rulebook');
	const entries = z.record(z.string(), z.unknown()).parse(firstRulebook['entries']);
	const entry = z.record(z.string(), z.unknown()).parse(entries['K001']);
	input['rulebooks'] = [
		{
			...firstRulebook,
			entries: {
				...entries,
				K001: {
					...entry,
					reference: { codec_boundary: 42n, numeric_string: '18446744073709551615' },
				},
			},
		},
		...rulebooks.slice(1),
	];
	const saved = outcomeSchema.parse(sdk.serialize({ kind: 'configuration', value: input }));
	saveSdkEvidence(path.join(evidence, 'codec-written.json'), saved);
	requireSdk(
		saved.kind === 'serialized' && typeof saved['json'] === 'string',
		'Writer refused configuration',
	);
	requireSdk(
		/"max_file_bytes"\s*:\s*1048576(?:\s*[,}])/.test(saved['json']),
		'Writer did not retain the full-width integer as a numeric token',
	);
	const readback = outcomeSchema.parse(
		await boundedSdkWait(deserialize(saved['json']), 'configuration readback'),
	);
	saveSdkEvidence(path.join(evidence, 'codec-readback.json'), readback);
	requireSdk(readback.kind === 'decoded', 'Written configuration did not decode');
	const taggedValue = z
		.object({ kind: z.literal('configuration'), value: z.unknown() })
		.parse(readback['value']);
	const configuration = z
		.object({ limits: z.object({ max_file_bytes: z.bigint() }) })
		.parse(taggedValue.value);
	requireSdk(
		configuration.limits.max_file_bytes === 1048576n,
		'Readback lost the full-width integer type or value',
	);
	requireSdk(Object.isFrozen(taggedValue.value), 'Decoded configuration is not frozen');
	const numberCases = [
		{
			name: 'independent-u64-max',
			json: saved['json'].replace(
				'"max_file_bytes":1048576',
				'"max_file_bytes":18446744073709551615',
			),
			kind: 'decoded',
		},
		{
			name: 'u64-overflow',
			json: saved['json'].replace(
				'"max_file_bytes":1048576',
				'"max_file_bytes":18446744073709551616',
			),
			kind: 'invalid',
			path: ['payload', 'value', 'limits', 'max_file_bytes'],
		},
		{
			name: 'integer-to-f64-rounding',
			json: saved['json'].replace('"min_probability":0.7', '"min_probability":9007199254740993'),
			kind: 'invalid',
			path: [
				'payload',
				'value',
				'filters',
				'purpose',
				'decision',
				'steps',
				'purpose',
				'min_probability',
			],
		},
		{
			name: 'structured-u64-overflow',
			json: saved['json'].replace('"codec_boundary":42', '"codec_boundary":18446744073709551616'),
			kind: 'invalid',
			path: [
				'payload',
				'value',
				'rulebooks',
				'0',
				'entries',
				'K001',
				'reference',
				'codec_boundary',
			],
		},
		{
			name: 'structured-i64-underflow',
			json: saved['json'].replace('"codec_boundary":42', '"codec_boundary":-9223372036854775809'),
			kind: 'invalid',
			path: [
				'payload',
				'value',
				'rulebooks',
				'0',
				'entries',
				'K001',
				'reference',
				'codec_boundary',
			],
		},
		{
			name: 'structured-i64-min',
			json: saved['json'].replace('"codec_boundary":42', '"codec_boundary":-9223372036854775808'),
			kind: 'decoded',
			expectedStructured: -9223372036854775808n,
		},
		{
			name: 'structured-u64-max',
			json: saved['json'].replace('"codec_boundary":42', '"codec_boundary":18446744073709551615'),
			kind: 'decoded',
			expectedStructured: 18446744073709551615n,
		},
	] as const;
	const numberOutcomes = await Promise.all(
		numberCases.map((testCase) => boundedSdkWait(deserialize(testCase.json), testCase.name)),
	);
	for (const [index, testCase] of numberCases.entries()) {
		requireSdk(
			testCase.json !== saved['json'],
			`${testCase.name}: fixture did not reach its field`,
		);
		const outcome = outcomeSchema.parse(numberOutcomes[index]);
		saveSdkEvidence(path.join(evidence, `codec-${testCase.name}.json`), {
			input: testCase.json,
			outcome,
		});
		requireSdk(outcome.kind === testCase.kind, `${testCase.name}: wrong outcome kind`);
		if ('path' in testCase) {
			requireSdk(
				outcome['reason'] === 'unsupported_number',
				`${testCase.name}: wrong refusal reason`,
			);
			const diagnostics = z
				.array(z.object({ json_path: z.array(z.string()) }))
				.parse(outcome['diagnostics']);
			requireSdk(
				diagnostics.some(
					(diagnostic) => JSON.stringify(diagnostic.json_path) === JSON.stringify(testCase.path),
				),
				`${testCase.name}: wrong diagnostic path`,
			);
		} else if ('expectedStructured' in testCase) {
			const value = z.record(z.string(), z.unknown()).parse(outcome['value']);
			const config = z.record(z.string(), z.unknown()).parse(value['value']);
			const books = z.array(z.record(z.string(), z.unknown())).parse(config['rulebooks']);
			const book = books[0];
			requireSdk(book, `${testCase.name}: decoded rulebook missing`);
			const decodedEntries = z.record(z.string(), z.unknown()).parse(book['entries']);
			const decodedEntry = z.record(z.string(), z.unknown()).parse(decodedEntries['K001']);
			const reference = z.record(z.string(), z.unknown()).parse(decodedEntry['reference']);
			requireSdk(
				reference['codec_boundary'] === testCase.expectedStructured,
				`${testCase.name}: structured integer changed`,
			);
			requireSdk(
				reference['numeric_string'] === '18446744073709551615',
				`${testCase.name}: numeric-looking string changed`,
			);
		} else {
			const decoded = z
				.object({
					kind: z.literal('configuration'),
					value: z.object({ limits: z.object({ max_file_bytes: z.bigint() }) }),
				})
				.parse(outcome['value']);
			requireSdk(
				decoded.value.limits.max_file_bytes === 18446744073709551615n,
				`${testCase.name}: wrong exact value`,
			);
		}
	}
	const cases = [
		{ name: 'malformed', json: '{', kind: 'invalid', reason: 'malformed_json' },
		{
			name: 'unknown-root',
			json: `{"contract_revision":${target.contractRevision},"payload":{"kind":"unknown","value":{}}}`,
			kind: 'invalid',
			reason: 'invalid_value',
		},
		{
			name: 'duplicate-struct-field',
			json: saved['json'].replace(
				'"name":"instruction_actor"',
				'"name":"instruction_actor","name":"duplicate"',
			),
			kind: 'invalid',
			reason: 'invalid_value',
		},
		{
			name: 'wrong-integer-type',
			json: saved['json'].replace('"max_file_bytes":1048576', '"max_file_bytes":"1048576"'),
			kind: 'invalid',
			reason: 'invalid_value',
		},
		{
			name: 'unknown-wrapper',
			json: `{"contract_revision":${target.contractRevision},"payload":null,"extra":true}`,
			kind: 'invalid',
			reason: 'invalid_document',
		},
		{
			name: 'missing-payload',
			json: `{"contract_revision":${target.contractRevision}}`,
			kind: 'invalid',
			reason: 'invalid_document',
		},
		{
			name: 'wrong-revision-before-payload',
			json: `{"contract_revision":${target.contractRevision + 1},"payload":{"kind":"not_a_root"}}`,
			kind: 'incompatible',
		},
	] as const;
	const outcomes = await Promise.all(
		cases.map((testCase) => boundedSdkWait(deserialize(testCase.json), testCase.name)),
	);
	for (const [index, testCase] of cases.entries()) {
		requireSdk(
			testCase.json !== saved['json'],
			`${testCase.name}: fixture did not change the document`,
		);
		const rawOutcome = outcomes[index];
		saveSdkEvidence(path.join(evidence, `codec-${testCase.name}.json`), rawOutcome);
		const outcome = outcomeSchema.parse(rawOutcome);
		requireSdk(outcome.kind === testCase.kind, `${testCase.name}: wrong outcome kind`);
		if ('reason' in testCase)
			requireSdk(outcome['reason'] === testCase.reason, `${testCase.name}: wrong refusal reason`);
		else
			requireSdk(
				outcome['expected_contract_revision'] === target.contractRevision &&
					outcome['actual_contract_revision'] === target.contractRevision + 1,
				`${testCase.name}: wrong revisions`,
			);
	}
	const unicodeText = saved['json'].replace(
		'"numeric_string":"18446744073709551615"',
		'"numeric_string":"\ud800"',
	);
	requireSdk(unicodeText !== saved['json'], 'Unicode fixture did not reach its field');
	const invalidUnicode = outcomeSchema.parse(
		await boundedSdkWait(deserialize(unicodeText), 'malformed Unicode input'),
	);
	saveSdkEvidence(path.join(evidence, 'codec-malformed-unicode.json'), invalidUnicode);
	if (invalidUnicode.kind === 'decoded') {
		const value = z.record(z.string(), z.unknown()).parse(invalidUnicode['value']);
		const config = z.record(z.string(), z.unknown()).parse(value['value']);
		const books = z.array(z.record(z.string(), z.unknown())).parse(config['rulebooks']);
		const book = books[0];
		requireSdk(book, 'Unicode decoded rulebook missing');
		const unicodeEntries = z.record(z.string(), z.unknown()).parse(book['entries']);
		const decodedEntry = z.record(z.string(), z.unknown()).parse(unicodeEntries['K001']);
		const reference = z.record(z.string(), z.unknown()).parse(decodedEntry['reference']);
		requireSdk(reference['numeric_string'] === '\ud800', 'Malformed Unicode was silently replaced');
	} else
		requireSdk(
			invalidUnicode.kind === 'invalid' || invalidUnicode.kind === 'failed',
			'Malformed Unicode did not return a typed refusal',
		);
	requireSdk(backend.captures.length === beforeRequests, 'Codec dispatched a model request');
	return {
		observed: [
			'configuration writer and Rust-backed readback preserve one full-width numeric field',
			'independent u64 maximum, structured overflows and exact-integer-to-f64 refusal with paths',
			'independent malformed, wrong-type, duplicate-field, strict-wrapper and revision-before-payload refusals',
			'codec calls made no controlled model requests',
		],
		gaps: [
			'independent signed and structured JSON boundaries and remaining number/path refusals',
			'generated widening counterfactuals, writer purity and no-engine joins',
		],
	};
}
