// A valid caller: compiles with no diagnostics under the package's strict options.
import { check, load, run } from '../../src/index.ts';
import type {
	AstMatcher,
	AuthoredLanguage,
	Configuration,
	EvidenceKindDefinition,
	EvidenceLanguage,
	FilterDefinition,
	HeadingAnchorScheme,
	HtmlAttributeName,
	HtmlEvidenceSettings,
	HtmlTagName,
	LoadedConfiguration,
	MarkdownEvidenceSettings,
	RuleEntry,
	RunOutcome,
	SdkFinding,
	TargetAnchor,
	TargetAnchorUnsupportedReason,
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

// Native HTML heading sections: one optional filter and the three section texts.
export const htmlSectionKind = {
	html_units: { mode: 'unit', units: ['section'], pattern: '^Parent' },
	fields: {
		complete: { from: 'section', text: 'full' },
		own: { from: 'section', text: 'own' },
		parent: { from: 'section', text: 'enclosing_own' },
	},
} satisfies EvidenceKindDefinition;

// Structured rule and filter data: exact integers are bigint, finite floats are number,
// and a returned value is read as either.
type StructuredValue = NonNullable<RuleEntry['reference']>[string];
export const structuredReference: RuleEntry['reference'] = {
	threshold: 2n,
	ratio: 0.5,
	wide: 18446744073709551615n,
	nested: { list: [1n, 2.5, 'text', null, true] },
};
export const filterReference: FilterDefinition['reference'] = { cap: -9223372036854775808n };
export function readStructured(value: StructuredValue): string {
	if (typeof value === 'bigint') return `exact ${value.toString()}`;
	if (typeof value === 'number') return `float ${value.toFixed(2)}`;
	return 'other';
}

// Upstream matcher fields: fix may be omitted or null; message and severity may be omitted.
export const matcherDefaults = {
	language: 'Markdown',
	rule: { kind: 'paragraph' },
	fix: null,
} satisfies AstMatcher;

// Full-width upstream range positions take exact bigint; small ones stay number.
export const wideRange = {
	language: 'Markdown',
	rule: {
		kind: 'paragraph',
		range: {
			start: { line: 9007199254740993n, column: 0 },
			end: { line: 9007199254740993n, column: 1n },
		},
	},
} satisfies AstMatcher;
export const smallRange = {
	language: 'Markdown',
	rule: {
		kind: 'paragraph',
		range: { start: { line: 1, column: 0 }, end: { line: 2, column: 4 } },
	},
} satisfies AstMatcher;

// A selection question is a noul judgment question.
export const selectedKind = {
	rule: 'paragraph',
	fields: { body: { from: 'remainder' } },
	select: {
		question: { type: 'noul', instructions: 'Is this paragraph a procedure?' },
		yes_at_or_above: 0.8,
		no_at_or_below: 0.2,
	},
} satisfies EvidenceKindDefinition;

// Native Markdown units at a rulebook kind: unit mode, known units, one optional filter.
export const markdownSectionKind = {
	markdown_units: { mode: 'unit', units: ['section'], starts_with: 'Steps' },
	fields: {
		full: { from: 'section', text: 'full' },
		own: { from: 'section', text: 'own' },
		parent: { from: 'section', text: 'enclosing_own' },
	},
} satisfies EvidenceKindDefinition;
export const markdownItemKind = {
	markdown_units: { mode: 'unit', units: ['paragraph', 'list_item'], pattern: '^-' },
	fields: { body: { from: 'remainder' } },
} satisfies EvidenceKindDefinition;

// An evidence kind names exactly one matcher; each of the four is a complete kind.
export const ruleKind = {
	rule: 'paragraph',
	fields: { body: { from: 'remainder' } },
} satisfies EvidenceKindDefinition;
export const inlineRuleKind = {
	inline_rule: 'kind: paragraph',
	fields: { body: { from: 'remainder' } },
} satisfies EvidenceKindDefinition;
export const htmlUnitsKind = {
	html_units: { mode: 'unit', units: ['section'] },
	fields: { body: { from: 'remainder' } },
} satisfies EvidenceKindDefinition;
export const markdownUnitsKind = {
	markdown_units: { mode: 'unit', units: ['section'] },
	fields: { body: { from: 'remainder' } },
} satisfies EvidenceKindDefinition;
// A kind built elsewhere and passed by reference is checked the same way.
const ruleKindValue = { rule: 'paragraph', fields: {} };
export const ruleKindByReference: EvidenceKindDefinition = ruleKindValue;

// Document conventions: typed settings, a reusable preset and every evidence form.
export const restoredConventions = {
	html: { skip_tags: ['ex-markdown'], text_attributes: { 'ex-mermaid': ['label'] } },
	markdown: { heading_anchors: 'github' },
} as const satisfies { html: HtmlEvidenceSettings; markdown: MarkdownEvidenceSettings };
const skippedTag: HtmlTagName = 'example-source';
const textAttribute: HtmlAttributeName = 'caption';
export const scheme: HeadingAnchorScheme = 'github';
export const customHtml: HtmlEvidenceSettings = {
	skip_tags: [skippedTag],
	text_attributes: { 'custom-diagram': [textAttribute] },
};
export const noScheme: MarkdownEvidenceSettings = { heading_anchors: null };
export const standardConventions: MarkdownEvidenceSettings = {};
export const configuredInline = {
	...configuration,
	evidence: { ...configuration.evidence, ...restoredConventions },
} satisfies Configuration;
export const configuredProject = {
	...configuration,
	evidence: {
		kind: 'project',
		config_path: 'evidence/sgconfig.yml',
		kinds: {},
		html: customHtml,
		markdown: { heading_anchors: scheme },
	},
} satisfies Configuration;
// A loaded value's captured evidence takes an edited convention like any other member.
export function withRestoredConventions(held: LoadedConfiguration): Configuration {
	return {
		...held.configuration,
		evidence: { ...held.configuration.evidence, html: customHtml, markdown: noScheme },
	};
}

// An unsupported anchor always names its closed reason; other states have none.
export function anchorReason(anchor: TargetAnchor): TargetAnchorUnsupportedReason | 'checked' {
	switch (anchor.status) {
		case 'found':
		case 'missing':
			return 'checked';
		case 'unsupported':
			return anchor.reason;
		default:
			return assertNever(anchor);
	}
}
export function reasonAdvice(reason: TargetAnchorUnsupportedReason): string {
	switch (reason) {
		case 'heading_scheme_not_configured':
			return 'set evidence.markdown.heading_anchors';
		case 'unsupported_document':
			return 'this document format has no anchors';
		default:
			return assertNever(reason);
	}
}
