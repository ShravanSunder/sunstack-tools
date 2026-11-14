/** Observe released section-scoped containment using the installed SDK and CLI. */
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
export const scopedHtml =
	"文<h2>Earlier</h2><img src='old'>\n<h2>Own</h2><img src='before'>\n<script type='text/markdown'>\n```mermaid\ngraph LR\n A --> B\n```\n</script>\n<ex-choice id='own'><img src='inner'>Choose</ex-choice><img src='after'>\n<h2>Empty</h2><ex-choice id='empty'>Choose</ex-choice>\n<h2>Later</h2><img src='later'>";
const fence = '```mermaid\ngraph LR\n A --> B\n```\n';
const ownChoice = "<ex-choice id='own'><img src='inner'>Choose</ex-choice>";
const emptyChoice = "<ex-choice id='empty'>Choose</ex-choice>";

function sourceCoordinate(part: string): [number, number] {
	return [part.split('\n').length, Array.from(part.slice(part.lastIndexOf('\n') + 1)).length + 1];
}

/** Assert raw fields and source coordinates from fixed source, not from producer output. */
export function requireScopedInsidePieces(value: unknown, client: 'sdk' | 'cli' = 'sdk'): void {
	const pieces = z.array(recordSchema).parse(value);
	const identity = createHash('sha256').update(scopedHtml).digest('hex');
	const selected = (kind: string, text: string): Record<string, unknown> => {
		const matches = pieces.filter((piece) => piece['kind'] === kind && piece['text'] === text);
		requireSdk(matches.length === 1 && matches[0], `Missing unique ${kind} subject`);
		return matches[0];
	};
	const expectField = (
		piece: Record<string, unknown>,
		name: string,
		expected: readonly string[],
	): void => {
		const field = recordSchema.parse(recordSchema.parse(piece['fields'])[name]);
		requireSdk(
			field['state'] === 'texts' && isDeepStrictEqual(field['values'], expected),
			`Scoped ${name} values differ`,
		);
		const spans = z.array(recordSchema).parse(field['spans']);
		requireSdk(spans.length === expected.length, `Scoped ${name} span count differs`);
		for (const [index, text] of expected.entries()) {
			const span = spans[index];
			requireSdk(span, `Scoped ${name} missing span`);
			const prefix = scopedHtml.slice(0, scopedHtml.indexOf(text));
			const end = prefix + text;
			const source = recordSchema.parse(span['source']);
			const [startLine, startColumn] = sourceCoordinate(prefix);
			const [endLine, endColumn] = sourceCoordinate(end);
			for (const [key, expectedNumber] of Object.entries({
				byte_start: Buffer.byteLength(prefix),
				byte_end: Buffer.byteLength(end),
				start_line: startLine,
				start_column: startColumn,
				end_line: endLine,
				end_column: endColumn,
			})) {
				const actual = span[key];
				requireSdk(
					typeof actual === (client === 'sdk' ? 'bigint' : 'number') &&
						BigInt(z.union([z.bigint(), z.number().int()]).parse(actual)) ===
							BigInt(expectedNumber),
					`Scoped ${name} ${key} differs`,
				);
			}
			requireSdk(
				source['path'] === 'docs/scoped.html' && source['content_identity'] === identity,
				`Scoped ${name} source identity differs`,
			);
		}
	};
	const own = selected('choice', ownChoice);
	const empty = selected('choice', emptyChoice);
	for (const [name, values] of Object.entries({
		html_before: ["<img src='before'>\n"],
		markdown_before: [fence],
		html_section: ["<img src='before'>\n", "<img src='inner'>Choose", "<img src='after'>\n"],
		markdown_section: [fence],
	})) {
		expectField(own, name, values);
		expectField(empty, name, []);
	}
	const body = recordSchema.parse(recordSchema.parse(own['fields'])['body']);
	requireSdk(
		body['state'] === 'text' && body['value'] === ownChoice,
		'Scoped context changed remainder',
	);
	const view = recordSchema.parse(own['model_view']);
	requireSdk(
		view['state'] === 'bounded' &&
			isDeepStrictEqual(recordSchema.parse(view['fields'])['markdown_before'], [fence]),
		'Scoped Markdown model view changed raw syntax',
	);
	const legacy = selected('legacy', ownChoice);
	for (const name of ['by_kind', 'by_rule']) expectField(legacy, name, ["<img src='inner'>Choose"]);
	const legacyEmpty = selected('legacy', emptyChoice);
	for (const name of ['by_kind', 'by_rule']) expectField(legacyEmpty, name, []);
}

export async function observeSdkScopedInside(options: {
	readonly sdk: SdkExports;
	readonly target: SdkTarget;
	readonly evidence: string;
	readonly backend: SdkControlledBackend;
}): Promise<JourneyResult> {
	const { sdk, target, evidence, backend } = options;
	requireSdk(
		sdk.inspect && sdk.serialize && sdk.deserialize,
		'Installed SDK lacks scoped-inside APIs',
	);
	const inspect = sdk.inspect;
	const serialize = sdk.serialize;
	const deserialize = sdk.deserialize;
	const root = path.join(evidence, 'scoped-inside-root');
	const definitions = path.join(evidence, 'scoped-inside-definitions');
	filesystem.mkdirSync(path.join(root, 'docs'), { recursive: true });
	filesystem.mkdirSync(path.join(definitions, 'rules'), { recursive: true });
	filesystem.writeFileSync(path.join(root, 'docs', 'scoped.html'), scopedHtml);
	const matchers = {
		choice: { language: 'Html', rule: { kind: 'element', regex: '^<ex-choice(?: |>)' } },
		image: { language: 'Html', rule: { kind: 'element', regex: '^<img(?: |>)' } },
		fence: { language: 'Markdown', rule: { kind: 'fenced_code_block' } },
	};
	const choiceFields = {
		html_before: { from: 'inside', kind: 'picture', scope: { section_prefix: 'outline' } },
		markdown_before: { from: 'inside', kind: 'diagram', scope: { section_prefix: 'outline' } },
		html_section: { from: 'inside', kind: 'picture', scope: { section: 'outline' } },
		markdown_section: { from: 'inside', kind: 'diagram', scope: { section: 'outline' } },
		body: { from: 'remainder' },
	};
	const kinds = {
		outline: { html_units: { mode: 'unit', units: ['section'] }, fields: {} },
		picture: { rule: 'image', fields: {} },
		diagram: { rule: 'fence', fields: {} },
		choice: { rule: 'choice', fields: choiceFields },
		legacy: {
			rule: 'choice',
			fields: {
				by_kind: { from: 'inside', kind: 'picture' },
				by_rule: { from: 'inside', rule: 'image' },
			},
		},
	};
	const ruleEntries = {
		K001: {
			name: 'scoped-picture',
			evidence: 'choice',
			severity: 'error',
			decision: {
				start: 'present',
				steps: {
					present: {
						mechanical_check: { tool: 'regex', field: 'html_before', pattern: '(?s).*' },
						yes: 'pass',
						['no']: { finding: 'No preceding picture.' },
					},
				},
			},
		},
	};
	const common = {
		marker_prefix: 'K',
		check: { files: ['docs/*.html'] },
		decision_model: { model: 'jev-latest', endpoint: backend.endpoint },
	};
	const html = { embedded_markdown: [{ tag: 'script', attributes: { type: 'text/markdown' } }] };
	const configuration = {
		...common,
		id: 'scoped-inside',
		base_directory: definitions,
		evidence: { kind: 'inline', utilities: {}, matchers, kinds, html },
		rulebooks: [
			{
				kind: 'inline',
				id: 'scoped-rules',
				markdown: 'A choice has a preceding picture. <!-- K001 -->\n',
				entries: ruleEntries,
			},
		],
	};
	const request = { root, scope: { kind: 'all' } };
	const before = backend.captures.length;
	const inspection = await boundedSdkWait(
		inspect(configuration, request),
		'scoped-inside inspection',
	);
	saveSdkEvidence(path.join(evidence, 'scoped-inside-inspection.json'), inspection);
	const outcome = recordSchema.parse(inspection);
	requireSdk(outcome['kind'] === 'inspected', 'Scoped-inside inspection did not finish');
	const report = recordSchema.parse(outcome['report']);
	requireSdk(
		recordSchema.parse(report['status'])['kind'] === 'clean',
		'Scoped-inside inspection was not clean',
	);
	requireScopedInsidePieces(report['pieces']);
	const saved = recordSchema.parse(serialize({ kind: 'configuration', value: configuration }));
	requireSdk(
		saved['kind'] === 'serialized' && typeof saved['json'] === 'string',
		'Scoped-inside configuration did not serialize',
	);
	const restored = recordSchema.parse(
		await boundedSdkWait(deserialize(saved['json']), 'scoped-inside codec'),
	);
	requireSdk(restored['kind'] === 'decoded', 'Scoped-inside configuration did not decode');
	const restoredDocument = recordSchema.parse(restored['value']);
	requireSdk(
		restoredDocument['kind'] === 'configuration',
		'Scoped-inside codec restored the wrong root',
	);
	const decoded = recordSchema.parse(restoredDocument['value']);
	const decodedKinds = recordSchema.parse(recordSchema.parse(decoded['evidence'])['kinds']);
	const decodedChoice = recordSchema.parse(decodedKinds['choice']);
	requireSdk(
		isDeepStrictEqual(
			recordSchema.parse(decodedChoice['fields'])['html_before'],
			choiceFields.html_before,
		),
		'Scoped field changed across current codec',
	);
	const previous = recordSchema.parse(
		await boundedSdkWait(
			deserialize(
				JSON.stringify({
					contract_revision: target.contractRevision - 1,
					payload: { kind: 'configuration', value: {} },
				}),
			),
			'previous codec revision',
		),
	);
	requireSdk(
		previous['kind'] === 'incompatible' &&
			previous['actual_contract_revision'] === target.contractRevision - 1 &&
			previous['expected_contract_revision'] === target.contractRevision,
		'Previous codec revision did not refuse',
	);
	const badFields = [
		{ from: 'inside', rule: 'image', scope: { section: 'outline' } },
		{ from: 'inside', scope: { section: 'outline' } },
		{ from: 'inside', kind: 'picture', scope: null },
		{ from: 'inside', kind: 'picture', scope: { section: 'outline', section_prefix: 'outline' } },
		{ from: 'inside', kind: 'picture', scope: { section: 'outline', typo: true } },
		{ from: 'inside', kind: 'picture', scope: { section: 'outline' }, extra: true },
	];
	const invalid = await Promise.all(
		badFields.map(async (field, index) => {
			const invalidConfiguration = {
				...configuration,
				evidence: {
					...configuration.evidence,
					kinds: { ...kinds, choice: { ...kinds.choice, fields: { bad: field } } },
				},
			};
			const wire = JSON.stringify({
				contract_revision: target.contractRevision,
				payload: { kind: 'configuration', value: invalidConfiguration },
			});
			const result = recordSchema.parse(
				await boundedSdkWait(deserialize(wire), `invalid scoped-inside ${index}`),
			);
			requireSdk(
				result['kind'] === 'invalid' && result['reason'] === 'invalid_value',
				`Invalid scoped-inside ${index} was not refused`,
			);
			return result;
		}),
	);
	const invalidKinds = [
		{
			...kinds,
			choice: {
				...kinds.choice,
				fields: { bad: { from: 'inside', kind: 'picture', scope: { section: 'missing' } } },
			},
		},
		{
			...kinds,
			outline: {
				...kinds.outline,
				fields: { cycle: { from: 'inside', kind: 'choice', scope: { section: 'outline' } } },
			},
		},
	];
	const admission = await Promise.all(
		invalidKinds.map(async (invalidKindsEntry, index) => {
			const result = recordSchema.parse(
				await boundedSdkWait(
					inspect(
						{ ...configuration, evidence: { ...configuration.evidence, kinds: invalidKindsEntry } },
						request,
					),
					`invalid scoped dependency ${index}`,
				),
			);
			requireSdk(
				result['kind'] === 'invalid' &&
					z.array(z.unknown()).parse(result['diagnostics']).length > 0,
				`Invalid scoped dependency ${index} was not refused`,
			);
			return result;
		}),
	);
	const evaluated = await boundedSdkWait(sdk.run(configuration, request), 'scoped-inside rule run');
	const run = recordSchema.parse(evaluated);
	requireSdk(run['kind'] === 'completed', 'Scoped rule did not finish');
	const runReport = recordSchema.parse(run['report']);
	requireSdk(
		recordSchema.parse(runReport['status'])['kind'] === 'findings' &&
			z.array(z.unknown()).parse(runReport['findings']).length === 1 &&
			z.array(z.unknown()).parse(runReport['results']).length === 2,
		'Scoped present/empty rule outcomes differ',
	);
	saveSdkEvidence(path.join(evidence, 'scoped-inside-controls.json'), {
		saved,
		restored,
		previous,
		invalid,
		admission,
		evaluated,
	});
	const cli = target.cli;
	const gaps: string[] = [];
	if (cli) {
		filesystem.writeFileSync(path.join(definitions, 'sgconfig.yml'), 'ruleDirs: [rules]\n');
		filesystem.writeFileSync(
			path.join(definitions, 'rules', 'matchers.yml'),
			"id: choice\nlanguage: Html\nrule: {kind: element, regex: '^<ex-choice(?: |>)'}\n---\nid: image\nlanguage: Html\nrule: {kind: element, regex: '^<img(?: |>)'}\n---\nid: fence\nlanguage: Markdown\nrule: {kind: fenced_code_block}\n",
		);
		filesystem.writeFileSync(
			path.join(definitions, 'rulebook.md'),
			'A choice has a preceding picture. <!-- K001 -->\n',
		);
		filesystem.writeFileSync(path.join(definitions, 'rules.json'), JSON.stringify(ruleEntries));
		const configPath = path.join(definitions, 'sunstack.lint.json');
		filesystem.writeFileSync(
			configPath,
			JSON.stringify({
				...common,
				rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
				evidence: { ast_grep: 'sgconfig.yml', kinds, html },
			}),
		);
		const env: NodeJS.ProcessEnv = { ...process.env, SUNSTACK_CREDENTIAL_STORE: 'off' };
		delete env['OPENROUTER_API_KEY'];
		const result = spawnSync(
			cli.binary,
			['lint', '--config', configPath, '--root', root, '--all', '--evidence'],
			{ cwd: root, env, encoding: 'utf8', timeout: 30000 },
		);
		saveSdkEvidence(path.join(evidence, 'scoped-inside-cli.json'), {
			status: result.status,
			signal: result.signal,
			stdout: result.stdout,
			stderr: result.stderr,
			error: result.error?.message,
		});
		requireSdk(
			!result.error && result.status === 0 && typeof result.stdout === 'string',
			'Scoped-inside CLI evidence failed',
		);
		const cliReport = recordSchema.parse(JSON.parse(result.stdout));
		requireSdk(
			recordSchema.parse(cliReport['status'])['kind'] === 'clean',
			'Scoped-inside CLI evidence was not clean',
		);
		requireScopedInsidePieces(cliReport['pieces'], 'cli');
	} else gaps.push('Scoped-inside CLI evidence: no pinned CLI supplied');
	requireSdk(
		backend.captures.length === before,
		'Scoped-inside observation dispatched a model request',
	);
	return {
		observed: [
			'scoped prefix/full raw HTML and cross-language Markdown pieces retain exact source identity, order and UTF-8 spans',
			'empty scope, unscoped rule/kind containment and unaffected remainder retain their meanings',
			'current codec preserves scoped fields and previous revision refuses',
			'invalid shapes and missing/cyclic dependencies are refused',
			'real mechanical rule distinguishes present from empty without model requests',
		],
		gaps,
	};
}
