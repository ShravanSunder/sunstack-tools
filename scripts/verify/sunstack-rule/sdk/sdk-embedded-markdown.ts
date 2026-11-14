/** Verify caller-configured Markdown islands through the installed SDK and CLI. */
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';
import { isDeepStrictEqual } from 'node:util';

import { z } from 'zod';

import { boundedSdkWait, type SdkControlledBackend } from './sdk-controlled-backend.ts';
import type { JourneyResult } from './sdk-journeys.ts';
import { requireSdk, saveSdkEvidence, type SdkExports, type SdkTarget } from './sdk-target.ts';

const recordSchema = z.record(z.string(), z.unknown());
const integerSchema = z.union([z.bigint(), z.number().int()]).transform((value) => BigInt(value));
const pieceSchema = z
	.object({
		kind: z.string(),
		text: z.string(),
		where: z
			.object({
				file: z.string(),
				span: z
					.object({
						byte_start: integerSchema,
						byte_end: integerSchema,
						start_line: integerSchema,
						start_column: integerSchema,
						end_line: integerSchema,
						end_column: integerSchema,
						source: z.object({ path: z.string(), content_identity: z.string() }).passthrough(),
					})
					.passthrough(),
			})
			.passthrough(),
	})
	.passthrough();

// The entity and mixed case exercise decoded exact values and case-insensitive names.
export const islandHtml =
	'<p>Before 文.</p>\r\n<SCRIPT TYPE="text&#47;markdown">\r\nParagraph café.\r\n</SCRIPT>\r\n<script type="TEXT/MARKDOWN">Decoy.</script>\r\n<p>After.</p>\r\n';
const selectedParagraph = 'Paragraph café.\r\n';
const htmlSelector = {
	embedded_markdown: [{ tag: 'script', attributes: { type: 'text/markdown' } }],
};

/** Independent fixture oracle shared by SDK, CLI and the pure sensitivity checks. */
export function requireSdkIslandPieces(
	value: unknown,
	configured: boolean,
	client: 'sdk' | 'cli' = 'sdk',
): void {
	const rawPieces = z.array(recordSchema).parse(value);
	const pieces = z.array(pieceSchema).parse(value);
	const expected = configured
		? [
				{ kind: 'html', text: '<p>Before 文.</p>' },
				{ kind: 'markdown_ast', text: selectedParagraph },
				{ kind: 'markdown_unit', text: selectedParagraph },
				{ kind: 'html', text: '<p>After.</p>' },
			]
		: [
				{ kind: 'html', text: '<p>Before 文.</p>' },
				{ kind: 'html', text: '<p>After.</p>' },
			];
	requireSdk(pieces.length === expected.length, 'Island piece count differs');
	const identity = createHash('sha256').update(islandHtml).digest('hex');
	for (const [index, item] of expected.entries()) {
		const piece = pieces[index];
		const rawPiece = rawPieces[index];
		requireSdk(
			piece && rawPiece && piece.kind === item.kind && piece.text === item.text,
			'Island order/text differs',
		);
		const rawSpan = recordSchema.parse(recordSchema.parse(rawPiece['where'])['span']);
		for (const field of [
			'byte_start',
			'byte_end',
			'start_line',
			'start_column',
			'end_line',
			'end_column',
		]) {
			requireSdk(
				typeof rawSpan[field] === (client === 'sdk' ? 'bigint' : 'number'),
				`Island ${client} ${field} has the wrong numeric type`,
			);
		}
		const start = islandHtml.indexOf(item.text);
		const prefix = islandHtml.slice(0, start);
		const span = piece.where.span;
		const expectedLine = BigInt(prefix.split('\n').length);
		const column = BigInt(Array.from(prefix.slice(prefix.lastIndexOf('\n') + 1)).length + 1);
		const lines = item.text.split('\n');
		const endLine = expectedLine + BigInt(lines.length - 1);
		const endColumn =
			lines.length === 1
				? column + BigInt(Array.from(item.text).length)
				: BigInt(Array.from(lines.at(-1) ?? '').length + 1);
		requireSdk(
			piece.where.file === 'docs/island.html' &&
				span.source.path === 'docs/island.html' &&
				span.source.content_identity === identity &&
				span.byte_start === BigInt(Buffer.byteLength(prefix)) &&
				span.byte_end === BigInt(Buffer.byteLength(prefix + item.text)) &&
				span.start_line === expectedLine &&
				span.start_column === column &&
				span.end_line === endLine &&
				span.end_column === endColumn,
			'Island original HTML identity or coordinates differ',
		);
	}
}

export async function observeSdkEmbeddedMarkdown(options: {
	readonly sdk: SdkExports;
	readonly target: SdkTarget;
	readonly evidence: string;
	readonly backend: SdkControlledBackend;
}): Promise<JourneyResult> {
	const { sdk, target, evidence, backend } = options;
	requireSdk(
		sdk.inspect && sdk.serialize && sdk.deserialize,
		'Installed SDK lacks island observation APIs',
	);
	const inspect = sdk.inspect;
	const serialize = sdk.serialize;
	const deserialize = sdk.deserialize;
	const root = path.join(evidence, 'island-root');
	const definitions = path.join(evidence, 'island-definitions');
	filesystem.mkdirSync(path.join(root, 'docs'), { recursive: true });
	filesystem.mkdirSync(path.join(definitions, 'rules'), { recursive: true });
	filesystem.writeFileSync(path.join(root, 'docs', 'island.html'), islandHtml);
	filesystem.writeFileSync(path.join(definitions, 'sgconfig.yml'), 'ruleDirs: [rules]\n');
	filesystem.writeFileSync(
		path.join(definitions, 'rules', 'html.yml'),
		"id: html\nlanguage: Html\nrule: {kind: element, regex: '^<p>'}\n",
	);
	filesystem.writeFileSync(
		path.join(definitions, 'rules', 'markdown.yml'),
		'id: markdown_ast\nlanguage: Markdown\nrule: {kind: paragraph}\n',
	);
	filesystem.writeFileSync(
		path.join(definitions, 'rulebook.md'),
		'A paragraph contains the expected word. <!-- K001 -->\n',
	);
	filesystem.writeFileSync(
		path.join(definitions, 'rules.json'),
		JSON.stringify({
			K001: {
				name: 'island-paragraph',
				evidence: 'markdown_ast',
				severity: 'error',
				decision: {
					start: 'contains',
					steps: {
						contains: {
							mechanical_check: { tool: 'contains', field: 'body', text: 'café' },
							yes: 'pass',
							['no']: { finding: 'Missing word.' },
						},
					},
				},
			},
		}),
	);
	const kinds = {
		html: { rule: 'html', fields: {} },
		markdown_ast: { rule: 'markdown_ast', fields: { body: { from: 'first_line' } } },
		markdown_unit: { markdown_units: { mode: 'unit', units: ['paragraph'] }, fields: {} },
	};
	const fileConfiguration = {
		marker_prefix: 'K',
		check: { files: ['docs/*.html'] },
		decision_model: { model: 'jev-latest', endpoint: backend.endpoint },
		evidence: { ast_grep: 'sgconfig.yml', kinds },
		rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
	};
	const configPath = path.join(definitions, 'sunstack.lint.json');
	filesystem.writeFileSync(configPath, JSON.stringify(fileConfiguration));
	const before = backend.captures.length;
	const loaded = recordSchema.parse(
		await boundedSdkWait(sdk.load({ directory: definitions }), 'load island defaults'),
	);
	requireSdk(loaded['kind'] === 'loaded', 'Island default configuration did not load');
	const loadedValue = recordSchema.parse(loaded['value']);
	const configuration = recordSchema.parse(structuredClone(loadedValue['configuration']));
	const originalEvidence = recordSchema.parse(configuration['evidence']);
	const request = { root, scope: { kind: 'all' } };
	const observe = async (
		label: string,
		input: unknown,
		configured: boolean,
	): Promise<Record<string, unknown>> => {
		const raw = await boundedSdkWait(inspect(input, request), `island ${label}`);
		saveSdkEvidence(path.join(evidence, `island-${label}.json`), raw);
		const outcome = recordSchema.parse(raw);
		requireSdk(outcome['kind'] === 'inspected', `Island ${label} did not finish`);
		const report = recordSchema.parse(outcome['report']);
		requireSdk(
			recordSchema.parse(report['status'])['kind'] === 'clean',
			`Island ${label} is not clean`,
		);
		requireSdkIslandPieces(report['pieces'], configured);
		return report;
	};
	const empty = {
		...configuration,
		evidence: { ...originalEvidence, html: { embedded_markdown: [] } },
	};
	await observe('omitted', configuration, false);
	await observe('empty', empty, false);
	const normalizedDefaults = await Promise.all(
		[configuration, empty].map(async (input) => {
			const document = recordSchema.parse(serialize({ kind: 'configuration', value: input }));
			requireSdk(
				document['kind'] === 'serialized' && typeof document['json'] === 'string',
				'Default island configuration did not serialize',
			);
			const decodedDefault = recordSchema.parse(
				await boundedSdkWait(deserialize(document['json']), 'restore default island settings'),
			);
			requireSdk(
				decodedDefault['kind'] === 'decoded',
				'Default island configuration did not decode',
			);
			const restoredDefault = recordSchema.parse(
				recordSchema.parse(decodedDefault['value'])['value'],
			);
			const defaultEvidence = recordSchema.parse(restoredDefault['evidence']);
			requireSdk(
				defaultEvidence['html'] === undefined,
				'Empty island defaults were not omitted by Rust serialization',
			);
			return decodedDefault;
		}),
	);
	requireSdk(
		isDeepStrictEqual(normalizedDefaults[0], normalizedDefaults[1]),
		'Omitted and empty island settings normalized to different configurations',
	);
	saveSdkEvidence(path.join(evidence, 'island-default-codec.json'), normalizedDefaults);
	await observe(
		'nonmatching',
		{
			...configuration,
			evidence: {
				...originalEvidence,
				html: { embedded_markdown: [{ tag: 'script', attributes: { type: 'TEXT/markdown' } }] },
			},
		},
		false,
	);
	const edited = { ...configuration, evidence: { ...originalEvidence, html: htmlSelector } };
	const encoded = recordSchema.parse(serialize({ kind: 'configuration', value: edited }));
	requireSdk(
		encoded['kind'] === 'serialized' && typeof encoded['json'] === 'string',
		'Island configuration did not serialize',
	);
	const encodedJson = z.string().parse(encoded['json']);
	const decoded = recordSchema.parse(
		await boundedSdkWait(deserialize(encodedJson), 'restore island configuration'),
	);
	requireSdk(decoded['kind'] === 'decoded', 'Island configuration did not deserialize');
	const document = recordSchema.parse(decoded['value']);
	requireSdk(document['kind'] === 'configuration', 'Island codec restored the wrong root');
	const restored = recordSchema.parse(document['value']);
	const restoredHtml = recordSchema.parse(recordSchema.parse(restored['evidence'])['html']);
	requireSdk(
		JSON.stringify(restoredHtml['embedded_markdown']) ===
			JSON.stringify(htmlSelector.embedded_markdown),
		'Island selector changed across codec',
	);
	saveSdkEvidence(path.join(evidence, 'island-load-codec.json'), { loaded, encoded, decoded });
	await observe('configured', restored, true);
	const run = await boundedSdkWait(sdk.run(restored, request), 'island mechanical run');
	saveSdkEvidence(path.join(evidence, 'island-run.json'), run);
	const runOutcome = recordSchema.parse(run);
	requireSdk(runOutcome['kind'] === 'completed', 'Island rule run did not finish');
	const runReport = recordSchema.parse(runOutcome['report']);
	requireSdk(
		recordSchema.parse(runReport['status'])['kind'] === 'clean' &&
			z.array(z.unknown()).parse(runReport['results']).length === 1,
		'Island rule did not evaluate the selected paragraph',
	);
	const invalidSelectors = [[{ tag: 'script', typo: true }], [{}], null];
	const validSelectorJson = JSON.stringify(htmlSelector.embedded_markdown);
	const invalid = await Promise.all(
		invalidSelectors.map(async (selector, index) => {
			const wire = encodedJson.replace(validSelectorJson, JSON.stringify(selector));
			requireSdk(
				wire !== encodedJson,
				`Invalid island settings ${index} did not reach the selector`,
			);
			const result = recordSchema.parse(
				await boundedSdkWait(deserialize(wire), `invalid island settings ${index}`),
			);
			requireSdk(
				result['kind'] === 'invalid' && result['reason'] === 'invalid_value',
				`Invalid island settings ${index} were not refused by Rust`,
			);
			return result;
		}),
	);
	const validHtmlJson = `"html":${JSON.stringify(htmlSelector)}`;
	const nullHtmlWire = encodedJson.replace(validHtmlJson, '"html":null');
	requireSdk(nullHtmlWire !== encodedJson, 'Null HTML settings did not reach the document');
	const nullHtml = recordSchema.parse(
		await boundedSdkWait(deserialize(nullHtmlWire), 'null HTML settings'),
	);
	requireSdk(
		nullHtml['kind'] === 'invalid' && nullHtml['reason'] === 'invalid_value',
		'Null HTML settings were not refused by Rust',
	);
	invalid.push(nullHtml);
	saveSdkEvidence(path.join(evidence, 'island-invalid-settings.json'), invalid);
	const cli = target.cli;
	const gaps: string[] = [];
	if (cli) {
		filesystem.writeFileSync(
			configPath,
			JSON.stringify({
				...fileConfiguration,
				evidence: { ...fileConfiguration.evidence, html: htmlSelector },
			}),
		);
		const env: NodeJS.ProcessEnv = { ...process.env, SUNSTACK_CREDENTIAL_STORE: 'off' };
		delete env['OPENROUTER_API_KEY'];
		const result = spawnSync(
			cli.binary,
			['lint', '--config', configPath, '--root', root, '--all', '--evidence'],
			{ cwd: root, env, encoding: 'utf8', timeout: 30000 },
		);
		saveSdkEvidence(path.join(evidence, 'island-cli.json'), {
			status: result.status,
			signal: result.signal,
			stdout: result.stdout,
			stderr: result.stderr,
			error: result.error?.message,
		});
		requireSdk(
			!result.error && result.status === 0 && typeof result.stdout === 'string',
			'Island CLI evidence failed',
		);
		const report = recordSchema.parse(JSON.parse(result.stdout));
		requireSdk(
			recordSchema.parse(report['status'])['kind'] === 'clean',
			'Island CLI evidence was not clean',
		);
		requireSdkIslandPieces(report['pieces'], true, 'cli');
	} else gaps.push('Configured-island CLI evidence: no pinned CLI supplied');
	requireSdk(backend.captures.length === before, 'Island observation dispatched a model request');
	return {
		observed: [
			'loaded defaults, edited configuration and current typed codec preserve configured HTML island selectors',
			'SDK and pinned CLI recover original HTML identity, Unicode/CRLF coordinates and source ordering for Markdown AST and native units',
			'omitted, empty and nonmatching selectors retain default selection',
			'unknown selector field, missing tag and null settings receive Rust codec refusals',
			'configured Markdown paragraph is evaluated by a real mechanical rule with zero controlled model requests',
		],
		gaps,
	};
}
