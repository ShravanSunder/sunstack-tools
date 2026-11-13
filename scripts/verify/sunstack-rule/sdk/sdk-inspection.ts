/** Drive static source inspection through the installed SDK, with independent source facts. */
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import { z } from 'zod';

import { boundedSdkWait, type SdkControlledBackend } from './sdk-controlled-backend.ts';
import { observeSdkInspectionCli } from './sdk-inspection-cli.ts';
import type { JourneyResult } from './sdk-journeys.ts';
import { requireSdk, saveSdkEvidence, type SdkExports, type SdkTarget } from './sdk-target.ts';

const recordSchema = z.record(z.string(), z.unknown());
const corpus = [
	{
		file: 'src/calls.ts',
		text: 'function run() {\n  console.log(value);\n}\n',
		match: 'console.log(value)',
		kind: 'typescript_call',
		line: 2n,
		column: 3n,
	},
	{
		file: 'src/calls.rs',
		text: 'fn run() {\n    send(value);\n}\n',
		match: 'send(value)',
		kind: 'rust_call',
		line: 2n,
		column: 5n,
	},
	{ file: 'src/empty.ts', text: 'const untouched = 1;\n' },
] as const;

export async function observeSdkInspection(options: {
	readonly sdk: SdkExports;
	readonly target: SdkTarget;
	readonly evidence: string;
	readonly backend: SdkControlledBackend;
}): Promise<JourneyResult> {
	const { sdk, target, evidence, backend } = options;
	requireSdk(sdk.inspect, 'Installed SDK does not export inspect');
	const root = path.join(evidence, 'inspection-root');
	filesystem.mkdirSync(path.join(root, 'src'), { recursive: true });
	for (const item of corpus) filesystem.writeFileSync(path.join(root, item.file), item.text);
	const configuration = {
		id: 'sdk-inspection',
		base_directory: evidence,
		marker_prefix: 'K',
		check: { files: ['src/**'] },
		decision_model: { model: 'jev-latest', endpoint: backend.endpoint },
		evidence: {
			kind: 'inline',
			utilities: {},
			matchers: {
				typescript_call: { language: 'TypeScript', rule: { pattern: 'console.log($A)' } },
				rust_call: { language: 'Rust', rule: { pattern: 'send($A)' } },
			},
			kinds: {
				typescript_call: { rule: 'typescript_call', fields: { body: { from: 'remainder' } } },
				rust_call: { rule: 'rust_call', fields: { body: { from: 'remainder' } } },
			},
		},
		filters: {},
		rulebooks: [],
	};
	const startRequests = backend.captures.length;
	const request = { root, scope: { kind: 'all' } };
	const inspected = await boundedSdkWait(
		sdk.inspect(configuration, request),
		'static SDK inspection',
	);
	saveSdkEvidence(path.join(evidence, 'inspection-sdk.json'), inspected);
	const outcome = recordSchema.parse(inspected);
	requireSdk(outcome['kind'] === 'inspected', 'Empty-rulebook inspection did not complete');
	const report = recordSchema.parse(outcome['report']);
	requireSdk(
		recordSchema.parse(report['status'])['kind'] === 'clean',
		'Static inspection was not clean',
	);
	const pieces = z.array(recordSchema).parse(report['pieces']);
	requireSdk(pieces.length === 2, 'Static inspection returned the wrong piece count');
	for (const item of corpus) {
		if (!('match' in item)) continue;
		const piece = pieces.find((entry) => entry['kind'] === item.kind);
		requireSdk(piece && piece['text'] === item.match, `Missing exact ${item.kind} text`);
		const location = recordSchema.parse(piece['where']);
		const span = recordSchema.parse(location['span']);
		const source = recordSchema.parse(span['source']);
		const offset = item.text.indexOf(item.match);
		requireSdk(location['file'] === item.file && offset >= 0, `Wrong ${item.kind} source`);
		requireSdk(
			span['byte_start'] === BigInt(offset) &&
				span['byte_end'] === BigInt(offset + item.match.length),
			`Wrong ${item.kind} bytes`,
		);
		requireSdk(
			span['start_line'] === item.line &&
				span['end_line'] === item.line &&
				span['start_column'] === item.column &&
				span['end_column'] === item.column + BigInt(item.match.length),
			`Wrong ${item.kind} lines or columns`,
		);
		requireSdk(
			source['content_identity'] === createHash('sha256').update(item.text).digest('hex'),
			`Wrong ${item.kind} source identity`,
		);
	}
	const fileCounts = z.array(recordSchema).parse(report['file_counts']);
	const emptyFile = fileCounts.find((entry) => entry['file'] === 'src/empty.ts');
	requireSdk(emptyFile, 'No-match file missing from coverage');
	const emptyKinds = recordSchema.parse(emptyFile['kinds']);
	const emptyCount = recordSchema.parse(emptyKinds['typescript_call']);
	requireSdk(
		emptyCount['matches'] === 0n && emptyCount['pieces'] === 0n && emptyCount['picked'] === 0n,
		'No-match file does not report zero counts',
	);
	const cliObservations = target.cli
		? observeSdkInspectionCli({ target, evidence, root, sdkReport: report })
		: [];
	const changedText = `${corpus[0].text}// Different source revision.\n`;
	filesystem.writeFileSync(path.join(root, corpus[0].file), changedText);
	const changed = await boundedSdkWait(
		sdk.inspect(configuration, request),
		'changed source inspection',
	);
	saveSdkEvidence(path.join(evidence, 'inspection-changed-source.json'), changed);
	const changedOutcome = recordSchema.parse(changed);
	requireSdk(changedOutcome['kind'] === 'inspected', 'Changed source inspection did not complete');
	const changedReport = recordSchema.parse(changedOutcome['report']);
	const changedPiece = z
		.array(recordSchema)
		.parse(changedReport['pieces'])
		.find((entry) => entry['kind'] === corpus[0].kind);
	requireSdk(
		changedPiece && changedPiece['text'] === corpus[0].match,
		'Changed source lost its unchanged match',
	);
	const changedSpan = recordSchema.parse(recordSchema.parse(changedPiece['where'])['span']);
	const changedSource = recordSchema.parse(changedSpan['source']);
	requireSdk(
		changedSource['content_identity'] === createHash('sha256').update(changedText).digest('hex'),
		'Changed source identity does not identify changed content',
	);
	requireSdk(
		changedSpan['byte_start'] === BigInt(corpus[0].text.indexOf(corpus[0].match)),
		'Unchanged match moved unexpectedly',
	);
	const checked = recordSchema.parse(
		await boundedSdkWait(sdk.check(configuration), 'legacy admission'),
	);
	requireSdk(checked['kind'] === 'invalid', 'Legacy check admitted an empty rulebook');
	const legacyRun = recordSchema.parse(
		await boundedSdkWait(sdk.run(configuration, request), 'legacy run admission'),
	);
	requireSdk(legacyRun['kind'] === 'invalid', 'Legacy run admitted an empty rulebook');
	const malformed = recordSchema.parse(
		await boundedSdkWait(
			sdk.inspect(
				{
					...configuration,
					evidence: {
						...configuration.evidence,
						kinds: { broken: { rule: 'missing', fields: {} } },
					},
				},
				request,
			),
			'inspection malformed definition',
		),
	);
	requireSdk(
		malformed['kind'] === 'invalid' &&
			z.array(z.unknown()).parse(malformed['diagnostics']).length > 0,
		'Inspection admitted a malformed definition',
	);
	const incomplete = recordSchema.parse(
		await boundedSdkWait(
			sdk.inspect({ ...configuration, limits: { max_file_bytes: 1n } }, request),
			'limited inspection',
		),
	);
	requireSdk(
		incomplete['kind'] === 'incomplete',
		'File size limit did not leave inspection incomplete',
	);
	const incompleteReport = recordSchema.parse(incomplete['report']);
	requireSdk(
		recordSchema.parse(incompleteReport['status'])['kind'] === 'incomplete',
		'Limited inspection report lost incomplete status',
	);
	requireSdk(
		z.array(z.unknown()).parse(incompleteReport['incomplete']).length > 0,
		'Limited inspection omitted its incomplete parts',
	);
	const unknown = recordSchema.parse(
		await boundedSdkWait(
			sdk.inspect(configuration, { ...request, evidence_kind: 'missing' }),
			'unknown evidence kind',
		),
	);
	requireSdk(unknown['kind'] === 'invalid', 'Unknown evidence kind was not refused');
	const cancellation = new AbortController();
	cancellation.abort();
	const cancelled = recordSchema.parse(
		await boundedSdkWait(
			sdk.inspect(configuration, { ...request, signal: cancellation.signal }),
			'pre-cancelled inspection',
		),
	);
	requireSdk(
		cancelled['kind'] === 'cancelled' && cancelled['report'] === null,
		'Pre-cancelled inspection did not preserve its ending',
	);
	saveSdkEvidence(path.join(evidence, 'inspection-controls.json'), {
		checked,
		legacyRun,
		malformed,
		incomplete,
		unknown,
		cancelled,
	});
	requireSdk(
		backend.captures.length === startRequests,
		'Static inspection dispatched a model request',
	);
	return {
		observed: [
			...cliObservations,
			'static TypeScript/Rust evidence text, source identity and spans',
			'no-match file coverage, empty-rulebook inspection and legacy check refusal',
			'unknown-kind refusal and pre-cancellation',
			'malformed definitions and legacy empty-rulebook run refused; file limit retained incomplete status and parts',
			'changed source bytes produce a distinct acquired content identity',
			'zero controlled model requests',
		],
		gaps: [
			...(!target.cli
				? ['CLI evidence parity, legacy admission and rule-violation witness: no CLI supplied']
				: []),
			'exclusions, CLI incomplete parity and in-flight cancellation',
			'exact incomplete/coverage detail inventory',
		],
	};
}
