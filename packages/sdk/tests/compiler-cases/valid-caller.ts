// A valid caller: compiles with no diagnostics under the package's strict options.
import { check, load, run } from '../../src/index.ts';
import type {
	AuthoredLanguage,
	Configuration,
	EvidenceLanguage,
	LoadedConfiguration,
	RunOutcome,
	SdkFinding,
} from '../../src/index.ts';

export const configuration = {
	id: 'compiler-valid',
	base_directory: '/abs/base',
	marker_prefix: 'K',
	check: { files: ['**/*.md'], ignore: ['vendor/**'] },
	decision_model: { model: 'jev-latest', endpoint: 'http://127.0.0.1:1/v1/systemone' },
	limits: { max_file_bytes: 18446744073709551615n, max_files: 100 },
	evidence: {
		kind: 'inline',
		utilities: {
			'paragraph-node': { language: 'Markdown', rule: { kind: 'paragraph' }, arguments: ['A'] },
		},
		matchers: {
			paragraph: {
				language: 'TypeScript',
				rule: {
					pattern: 'console.log($A)',
					inside: { kind: 'function_declaration', stopBy: 'end' },
				},
				constraints: { A: { kind: 'identifier' } },
				fix: 'console.info($A)',
				message: 'Use info.',
				severity: 'warning',
				labels: { A: { style: 'primary', message: 'here' } },
				files: ['src/**', { glob: 'lib/**', caseInsensitive: true }],
				metadata: {
					owner: 'docs',
					tags: ['a', null],
					nested: { level: 1 },
					wide: 18446744073709551615n,
				},
			},
		},
		kinds: {
			paragraph: {
				rule: 'paragraph',
				fields: { body: { from: 'remainder' }, first: { from: 'first_line' } },
			},
		},
	},
	rulebooks: [
		{ kind: 'files', markdown: 'rulebook.md', entries: 'rules.json' },
		{
			kind: 'inline',
			id: 'inline-standards',
			markdown: 'Rule. <!-- K001 -->\n',
			entries: {
				K001: {
					name: 'rule_one',
					evidence: 'paragraph',
					severity: 'warn',
					decision: {
						start: 'present',
						steps: {
							present: {
								mechanical_check: { tool: 'contains_field', field: 'body', needle: 'actor' },
								yes: 'pass',
								no: { finding: 'Name the actor.', at: 'body' },
							},
						},
					},
				},
			},
		},
	],
} satisfies Configuration;

// Every canonical authored language spelling is accepted, and the union is exhaustive.
export const authoredLanguages = [
	'Bash',
	'C',
	'Cpp',
	'CSharp',
	'Css',
	'Dart',
	'Elixir',
	'Go',
	'Haskell',
	'Hcl',
	'Html',
	'Java',
	'JavaScript',
	'Json',
	'Kotlin',
	'Lua',
	'Markdown',
	'Nix',
	'Php',
	'Python',
	'Ruby',
	'Rust',
	'Scala',
	'Solidity',
	'Swift',
	'Tsx',
	'TypeScript',
	'Yaml',
	'Zig',
] as const satisfies readonly AuthoredLanguage[];
type MissingAuthored = Exclude<AuthoredLanguage, (typeof authoredLanguages)[number]>;
export const authoredComplete: [MissingAuthored] extends [never] ? true : false = true;

// Result languages are the separate snake_case union.
export function describeResultLanguage(language: EvidenceLanguage): string {
	return language === 'c_sharp' || language === 'type_script' ? 'braces' : language;
}

function assertNever(value: never): never {
	throw new Error(`unhandled ${String(value)}`);
}

export function summarize(outcome: RunOutcome): string {
	switch (outcome.kind) {
		case 'completed': {
			const status = outcome.report.status;
			switch (status.kind) {
				case 'clean':
				case 'nothing_selected':
				case 'findings':
				case 'uncertain':
					return `${status.kind}: ${outcome.report.findings.length}`;
				default:
					return assertNever(status);
			}
		}
		case 'invalid':
			return outcome.report === null
				? (outcome.diagnostics[0]?.code ?? 'invalid')
				: outcome.report.status.kind;
		case 'unavailable':
			return outcome.diagnostics.map((diagnostic) => diagnostic.code).join(',');
		case 'incomplete':
			return outcome.report.status.kind;
		case 'cancelled':
			return outcome.report?.status.kind ?? 'cancelled before dispatch';
		case 'failed':
			return `${outcome.failure.phase}/${outcome.failure.effects}/${outcome.failure.reason}`;
		default:
			return assertNever(outcome);
	}
}

export function firstLine(finding: SdkFinding): bigint {
	return finding.line;
}

export async function loadEditRun(directory: string, root: string): Promise<string> {
	const loaded = await load({ directory });
	if (loaded.kind !== 'loaded') return loaded.kind;
	const held: LoadedConfiguration = loaded.value;
	const changed = {
		...held.configuration,
		limits: { ...held.configuration.limits, max_files: 10 },
	} satisfies Configuration;
	const admission = await check(changed, { signal: AbortSignal.timeout(1000) });
	if (admission.kind === 'valid') {
		const outcome = await run(held, {
			root,
			scope: { kind: 'since', rev: 'HEAD~1' },
			paths: ['docs'],
		});
		return summarize(outcome);
	}
	return admission.kind;
}
