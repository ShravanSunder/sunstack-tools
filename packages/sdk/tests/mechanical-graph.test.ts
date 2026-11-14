// The generated evidence graph executed through the real addon: every authored
// language, document and target fields, anchors and whole-token field checks.
import assert from 'node:assert/strict';
import { rmSync } from 'node:fs';
import { test } from 'node:test';

import type {
	AstMatcher,
	AuthoredLanguage,
	Configuration,
	EvidenceKindDefinition,
	RuleEntry,
	RunOutcome,
	SdkReport,
} from '../src/index.ts';
import { run } from '../src/index.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

function reportOf(outcome: RunOutcome): SdkReport {
	if (outcome.kind === 'completed' || outcome.kind === 'incomplete') return outcome.report;
	throw new Error(
		`run did not reach a report: ${JSON.stringify(outcome, (_key, value: unknown) =>
			typeof value === 'bigint' ? value.toString() : value,
		)}`,
	);
}

/** One probe file per authored language: extension, content holding the token, and the grammar's root kind. */
const languageProbes = [
	['Bash', 'sh', '# sunstackprobe\n', 'program'],
	['C', 'c', '// sunstackprobe\n', 'translation_unit'],
	['Cpp', 'cpp', '// sunstackprobe\n', 'translation_unit'],
	['CSharp', 'cs', '// sunstackprobe\n', 'compilation_unit'],
	['Css', 'css', '/* sunstackprobe */\n', 'stylesheet'],
	['Dart', 'dart', '// sunstackprobe\n', 'source_file'],
	['Elixir', 'ex', '# sunstackprobe\n', 'source'],
	['Go', 'go', '// sunstackprobe\n', 'source_file'],
	['Haskell', 'hs', '-- sunstackprobe\n', 'haskell'],
	['Hcl', 'hcl', '# sunstackprobe\n', 'config_file'],
	['Html', 'html', '<!-- sunstackprobe -->\n', 'document'],
	['Java', 'java', '// sunstackprobe\n', 'program'],
	['JavaScript', 'js', '// sunstackprobe\n', 'program'],
	['Json', 'json', '{"sunstackprobe": 1}\n', 'document'],
	['Kotlin', 'kt', '// sunstackprobe\n', 'source_file'],
	['Lua', 'lua', '-- sunstackprobe\n', 'chunk'],
	['Markdown', 'md', 'sunstackprobe\n', 'document'],
	['Nix', 'nix', '# sunstackprobe\n', 'source_code'],
	['Php', 'php', '<?php // sunstackprobe\n', 'program'],
	['Python', 'py', '# sunstackprobe\n', 'module'],
	['Ruby', 'rb', '# sunstackprobe\n', 'program'],
	['Rust', 'rs', '// sunstackprobe\n', 'source_file'],
	['Scala', 'scala', '// sunstackprobe\n', 'compilation_unit'],
	['Solidity', 'sol', '// sunstackprobe\n', 'source_file'],
	['Swift', 'swift', '// sunstackprobe\n', 'source_file'],
	['Tsx', 'tsx', '// sunstackprobe\n', 'program'],
	['TypeScript', 'ts', '// sunstackprobe\n', 'program'],
	['Yaml', 'yml', '# sunstackprobe\n', 'stream'],
	['Zig', 'zig', '// sunstackprobe\n', 'source_file'],
] as const satisfies readonly (readonly [AuthoredLanguage, string, string, string])[];
// A language added to the generated union without a probe fails to compile here.
const probesComplete: [Exclude<AuthoredLanguage, (typeof languageProbes)[number][0]>] extends [
	never,
]
	? true
	: false = true;

await test('every authored language executes its matcher through the addon', async () => {
	const base = ownedDirectory('languages-base');
	const repository = ownedDirectory('languages');
	try {
		const languages = languageProbes.map(([language]) => language);
		// Exactly the generated authored union, once each.
		assert.ok(probesComplete);
		assert.equal(new Set(languages).size, 29);
		const matchers: Record<string, AstMatcher> = {};
		const kinds: Record<string, EvidenceKindDefinition> = {};
		const entries: Record<string, RuleEntry> = {};
		const markers: string[] = [];
		const files: Record<string, string> = {};
		for (const [index, [language, extension, content, root]] of languageProbes.entries()) {
			const name = `probe_${language.toLowerCase()}`;
			const marker = `K${String(index + 1).padStart(3, '0')}`;
			matchers[name] = { language, rule: { kind: root, regex: 'sunstackprobe' } };
			kinds[name] = { rule: name, fields: { line: { from: 'first_line' } } };
			entries[marker] = {
				name,
				evidence: name,
				severity: 'error',
				decision: {
					start: 'seen',
					steps: {
						seen: {
							mechanical_check: { tool: 'contains', field: 'line', text: 'sunstackprobe' },
							yes: { finding: `Seen by ${language}.` },
							no: 'pass',
						},
					},
				},
			};
			markers.push(`${language} probes are seen. <!-- ${marker} -->`);
			files[`probe/${name}.${extension}`] = content;
		}
		writeFiles(repository.path, files);
		const configuration: Configuration = {
			id: 'languages',
			base_directory: base.path,
			marker_prefix: 'K',
			check: { files: ['probe/*'] },
			decision_model: { model: 'jev-latest' },
			evidence: { kind: 'inline', utilities: {}, matchers, kinds },
			rulebooks: [
				{ kind: 'inline', id: 'language-rules', markdown: `${markers.join('\n')}\n`, entries },
			],
		};

		const report = reportOf(
			await run(configuration, { root: repository.path, scope: { kind: 'all' } }),
		);
		assert.equal(report.status.kind, 'findings');
		// Each language's file was parsed by its own grammar and only its own matcher applied.
		for (const language of languages) {
			const name = `probe_${language.toLowerCase()}`;
			const counted = report.file_counts.find((row) => row.file.startsWith(`probe/${name}.`));
			assert.ok(counted, `${language} file was examined`);
			// The grammar's root node holds the token: exactly one match, from this language's matcher only.
			assert.deepEqual(
				Object.entries(counted.kinds)
					.filter(([, count]) => count.matches > 0n)
					.map(([kind, count]) => [kind, count.matches]),
				[[name, 1n]],
				`${language} matched only through its own grammar`,
			);
			assert.ok(
				report.findings.some(
					(finding) => finding.file === counted.file && finding.message === `Seen by ${language}.`,
				),
				`${language} piece reached its rule`,
			);
		}
	} finally {
		base.remove();
		repository.remove();
	}
});

function htmlGraphConfiguration(base: string): Configuration {
	return {
		id: 'html-graph',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['docs/*.html'] },
		decision_model: { model: 'jev-latest' },
		evidence: {
			kind: 'inline',
			utilities: {},
			matchers: {
				paragraph: { language: 'Html', rule: { kind: 'element', regex: '^<p' } },
				whole: { language: 'Html', rule: { kind: 'document' } },
			},
			kinds: {
				paragraph: {
					rule: 'paragraph',
					fields: {
						body: { from: 'remainder' },
						identifier: { from: 'capture', regex: 'id="(R[0-9]+)"' },
						coverage: { from: 'document', path: '../coverage.html', rule: 'whole' },
						links: { from: 'target', regex: 'href="([^"]+)"', resolve: 'path' },
					},
				},
			},
		},
		rulebooks: [
			{
				kind: 'inline',
				id: 'graph-rules',
				markdown: 'Each requirement links to its coverage. <!-- K001 -->\n',
				entries: {
					K001: {
						name: 'coverage',
						evidence: 'paragraph',
						severity: 'error',
						decision: {
							start: 'anchor',
							steps: {
								anchor: {
									mechanical_check: { tool: 'anchor_exists', field: 'links' },
									yes: 'coverage',
									no: { finding: 'Dangling anchor.' },
								},
								coverage: {
									mechanical_check: {
										tool: 'contains_field',
										field: 'coverage',
										needle: 'identifier',
									},
									yes: 'raw',
									no: { finding: 'Missing coverage.' },
								},
								raw: {
									mechanical_check: { tool: 'regex', field: 'body', pattern: '<b>' },
									yes: 'pass',
									no: { finding: 'Mechanical text lost source markup.' },
								},
							},
						},
					},
				},
			},
		],
	};
}

/** Finding messages by piece line, and each piece's first link state. */
function graphObservation(report: SdkReport): {
	findings: Record<string, string>;
	links: Record<string, unknown>;
} {
	const findings: Record<string, string> = {};
	for (const finding of report.findings) findings[String(finding.line)] = finding.message;
	const links: Record<string, unknown> = {};
	for (const evidence of Object.values(report.piece_evidence)) {
		const field = evidence.piece.fields['links'];
		const line = String(evidence.piece.where.first_line);
		if (field?.state === 'targets') {
			const [target] = field.values;
			links[line] = target?.exists === true ? (target.anchor?.status ?? 'none') : 'absent';
		} else {
			links[line] = field?.state === 'unavailable' ? field.reason : field?.state;
		}
	}
	return { findings, links };
}

await test('document and target fields, anchors and whole-token checks execute natively', async () => {
	const repository = ownedDirectory('html-graph');
	try {
		writeFiles(repository.path, {
			'docs/source.html': [
				'<p id="R1"><a href="../coverage.html#covered">R1</a> <b>x</b></p>',
				'<p id="R2"><a href="../coverage.html#absent">R2</a> <b>y</b></p>',
				'<p id="R3"><a href="../notes.txt#part">R3</a> <b>z</b></p>',
				'<p id="R4"><a href="../../outside.html#a">R4</a> <b>w</b></p>',
				'',
			].join('\n'),
			'coverage.html': '<p id="covered">R10 R2 R3 R4</p>',
			'notes.txt': 'part\n',
		});
		const configuration = htmlGraphConfiguration(repository.path);
		const runAll = async (): Promise<SdkReport> =>
			reportOf(await run(configuration, { root: repository.path, scope: { kind: 'all' } }));

		const nearMiss = await runAll();
		assert.deepEqual(graphObservation(nearMiss), {
			// R1 is not a whole token of "R10"; R2's anchor is missing.
			findings: { '1': 'Missing coverage.', '2': 'Dangling anchor.' },
			links: { '1': 'found', '2': 'missing', '3': 'unsupported', '4': 'outside_root' },
		});
		// An unsupported anchor and an outside-root target leave the check undecided.
		assert.equal(nearMiss.status.kind, 'incomplete');
		assert.deepEqual(
			nearMiss.incomplete.map((entry) => [entry.field, entry.reason.kind]),
			[
				['links', 'evidence_unavailable'],
				['links', 'evidence_unavailable'],
			],
		);
		const coverage = Object.values(nearMiss.piece_evidence)[0]?.piece.fields['coverage'];
		assert.equal(coverage?.state, 'texts');
		if (coverage?.state === 'texts') {
			assert.deepEqual(coverage.values, ['<p id="covered">R10 R2 R3 R4</p>']);
			assert.equal(
				coverage.spans[0]?.source.kind === 'document' && coverage.spans[0].source.path,
				'coverage.html',
			);
		}

		writeFiles(repository.path, { 'coverage.html': '<p id="covered">(R1) R2 R3 R4</p>' });
		assert.deepEqual(graphObservation(await runAll()).findings, { '2': 'Dangling anchor.' });

		rmSync(`${repository.path}/coverage.html`);
		const missingDocument = graphObservation(await runAll());
		assert.equal(missingDocument.links['1'], 'absent');
		assert.equal(missingDocument.findings['1'], 'Dangling anchor.');
	} finally {
		repository.remove();
	}
});
