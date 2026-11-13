import { createHash } from 'node:crypto';
/** Observe the released section_prefix field through installed inspection and rule execution. */
import filesystem from 'node:fs';
import path from 'node:path';

import { z } from 'zod';

import { boundedSdkWait, type SdkControlledBackend } from './sdk-controlled-backend.ts';
import type { JourneyResult } from './sdk-journeys.ts';
import { requireSdk, saveSdkEvidence, type SdkExports } from './sdk-target.ts';

const recordSchema = z.record(z.string(), z.unknown());
const source =
	'// café\nfunction run() {\n  const value = 1;\n  console.log(value);\n  function nested() {\n    console.log(other);\n  }\n}\n';
const outside = 'console.log(outside);\n';

/** Check exact independent fixture facts; product observation supplies the actual pieces. */
export function requireSdkSectionPrefixPieces(pieces: unknown): void {
	const subjects = z
		.array(recordSchema)
		.parse(pieces)
		.filter((piece) => piece['kind'] === 'subject');
	requireSdk(subjects.length === 3, 'section_prefix subject count differs');
	for (const { expression, start, startLine, startColumn, endLine, endColumn } of [
		{
			expression: 'console.log(value)',
			start: 'function run()',
			startLine: 2n,
			startColumn: 1n,
			endLine: 4n,
			endColumn: 3n,
		},
		{
			expression: 'console.log(other)',
			start: 'function nested()',
			startLine: 5n,
			startColumn: 3n,
			endLine: 6n,
			endColumn: 5n,
		},
	]) {
		const sectionStart = source.indexOf(start);
		const piece = subjects.find((item) => item['text'] === expression);
		requireSdk(piece, `Missing section_prefix subject ${expression}`);
		const field = recordSchema.parse(recordSchema.parse(piece['fields'])['preceding']);
		const end = source.indexOf(expression);
		requireSdk(
			field['state'] === 'text' && field['value'] === source.slice(sectionStart, end),
			`Wrong exact prefix for ${expression}`,
		);
		const spans = z.array(recordSchema).parse(field['spans']);
		requireSdk(
			spans.length === 1 &&
				spans[0]?.['byte_start'] === BigInt(Buffer.byteLength(source.slice(0, sectionStart))) &&
				spans[0]?.['byte_end'] === BigInt(Buffer.byteLength(source.slice(0, end))),
			`Wrong UTF-8 prefix span for ${expression}`,
		);
		const prefixSpan = spans[0];
		requireSdk(
			prefixSpan &&
				prefixSpan['start_line'] === startLine &&
				prefixSpan['start_column'] === startColumn &&
				prefixSpan['end_line'] === endLine &&
				prefixSpan['end_column'] === endColumn,
			`Wrong prefix line/column for ${expression}`,
		);
		requireSdk(
			recordSchema.parse(prefixSpan['source'])['content_identity'] ===
				createHash('sha256').update(source).digest('hex'),
			`Wrong prefix source identity for ${expression}`,
		);
		const empty = recordSchema.parse(recordSchema.parse(piece['fields'])['empty_prefix']);
		const emptySpans = z.array(recordSchema).parse(empty['spans']);
		requireSdk(
			empty['state'] === 'text' &&
				empty['value'] === '' &&
				emptySpans.length === 1 &&
				emptySpans[0]?.['byte_start'] === BigInt(Buffer.byteLength(source.slice(0, end))) &&
				emptySpans[0]?.['byte_end'] === BigInt(Buffer.byteLength(source.slice(0, end))),
			'Equal-range section did not retain an empty prefix',
		);
	}
	const outsidePiece = subjects.find((item) => item['text'] === 'console.log(outside)');
	requireSdk(outsidePiece, 'Missing outside-section subject');
	const absent = recordSchema.parse(recordSchema.parse(outsidePiece['fields'])['preceding']);
	requireSdk(
		absent['state'] === 'unavailable' && absent['reason'] === 'unreadable',
		'Missing container did not produce unavailable/unreadable',
	);
}

export async function observeSdkSectionPrefix(options: {
	readonly sdk: SdkExports;
	readonly evidence: string;
	readonly backend: SdkControlledBackend;
}): Promise<JourneyResult> {
	const { sdk, evidence, backend } = options;
	requireSdk(sdk.inspect, 'Installed SDK does not export inspect');
	const inspect = sdk.inspect;
	const root = path.join(evidence, 'section-prefix-root');
	filesystem.mkdirSync(path.join(root, 'src'), { recursive: true });
	filesystem.writeFileSync(path.join(root, 'src', 'contained.ts'), source);
	filesystem.writeFileSync(path.join(root, 'src', 'outside.ts'), outside);
	const kinds = {
		outline: { rule: 'functions', fields: {} },
		same_call: { rule: 'calls', fields: {} },
		subject: {
			rule: 'calls',
			fields: {
				preceding: { from: 'section_prefix', kind: 'outline' },
				empty_prefix: { from: 'section_prefix', kind: 'same_call' },
			},
		},
	};
	const configuration = {
		id: 'section-prefix',
		base_directory: evidence,
		marker_prefix: 'K',
		check: { files: ['src/**'] },
		decision_model: { model: 'jev-latest', endpoint: backend.endpoint },
		evidence: {
			kind: 'inline',
			utilities: {},
			matchers: {
				functions: { language: 'TypeScript', rule: { kind: 'function_declaration' } },
				calls: { language: 'TypeScript', rule: { pattern: 'console.log($A)' } },
			},
			kinds,
		},
		rulebooks: [],
	};
	const request = { root, scope: { kind: 'all' } };
	const before = backend.captures.length;
	const raw = await boundedSdkWait(inspect(configuration, request), 'section prefix inspection');
	saveSdkEvidence(path.join(evidence, 'section-prefix-inspection.json'), raw);
	const outcome = recordSchema.parse(raw);
	requireSdk(outcome['kind'] === 'inspected', 'section_prefix inspection did not finish');
	const report = recordSchema.parse(outcome['report']);
	requireSdkSectionPrefixPieces(report['pieces']);
	const badSources = [
		{ from: 'section_prefix' },
		{ from: 'section_prefix', kind: null },
		{ from: 'section_prefix', kind: 'outline', text: 'full' },
	];
	const malformed = await Promise.all(
		badSources.map(async (preceding, index) => {
			const result = await boundedSdkWait(
				inspect(
					{
						...configuration,
						evidence: {
							...configuration.evidence,
							kinds: { ...kinds, subject: { ...kinds.subject, fields: { preceding } } },
						},
					},
					request,
				),
				`invalid section prefix ${index}`,
			);
			const parsed = recordSchema.parse(result);
			requireSdk(
				parsed['kind'] === 'invalid' &&
					z.array(z.unknown()).parse(parsed['diagnostics']).length > 0,
				`Invalid section_prefix ${index} was not refused`,
			);
			return result;
		}),
	);
	const runConfiguration = {
		...configuration,
		check: { files: ['src/contained.ts'] },
		rulebooks: [
			{
				kind: 'inline',
				id: 'prefix-rules',
				markdown: 'Prefix contains the function. <!-- K001 -->\n',
				entries: {
					K001: {
						name: 'prefix',
						evidence: 'subject',
						severity: 'error',
						decision: {
							start: 'present',
							steps: {
								present: {
									mechanical_check: {
										tool: 'contains',
										field: 'preceding',
										text: 'function ',
									},
									yes: 'pass',
									['no']: { finding: 'Function prefix missing.' },
								},
							},
						},
					},
				},
			},
		],
	};
	const run = await boundedSdkWait(sdk.run(runConfiguration, request), 'section prefix rule run');
	saveSdkEvidence(path.join(evidence, 'section-prefix-controls.json'), { malformed, run });
	const runOutcome = recordSchema.parse(run);
	requireSdk(runOutcome['kind'] === 'completed', 'section_prefix rule did not complete');
	const runReport = recordSchema.parse(runOutcome['report']);
	requireSdk(
		recordSchema.parse(runReport['status'])['kind'] === 'clean' &&
			z.array(z.unknown()).parse(runReport['results']).length === 2,
		'section_prefix rule did not evaluate both subjects cleanly',
	);
	requireSdk(
		backend.captures.length === before,
		'Static section_prefix inspection or rule dispatched a model request',
	);
	return {
		observed: [
			'installed section_prefix returns exact distinct prefixes and UTF-8 spans, empty and unavailable cases',
			'missing/null kind and unknown member refused; both contained subjects evaluated through run',
			'zero controlled model requests',
		],
		gaps: [
			'crossing, stale source and scan-limit cases remain native-test joins',
			'section_prefix CLI parity and compiler negatives remain separate joins',
		],
	};
}
