// Each marked line must fail with exactly the named diagnostic; nothing else may fail.
import { check, deserialize, inspect, load, run, serialize } from '../../src/index.ts';
import type {
	AstMatcher,
	Configuration,
	EvidenceKindDefinition,
	EvidenceLanguage,
	FieldSource,
	FilterDefinition,
	HtmlEvidenceSettings,
	LoadedConfiguration,
	MarkdownEvidenceSettings,
	RuleEntry,
	RunOutcome,
	TargetAnchor,
	TargetAnchorUnsupportedReason,
} from '../../src/index.ts';
import { configuration } from './valid-caller.ts';

export const unknownTopLevelKey = {
	...configuration,
	// expect TS2353
	unknown_setting: true,
} satisfies Configuration;

export const wrongEvidenceVariant = {
	...configuration,
	evidence: {
		// expect TS2820
		kind: 'inlined',
		matchers: {},
		kinds: {},
	},
} satisfies Configuration;

export const misspelledMatcherProperty = {
	...configuration,
	evidence: {
		kind: 'inline',
		matchers: {
			// expect TS2561
			paragraph: { language: 'Markdown', rul: { kind: 'paragraph' } },
		},
		kinds: {},
	},
} satisfies Configuration;

export const unsupportedLanguage = {
	...configuration,
	evidence: {
		kind: 'inline',
		matchers: {
			// expect TS2322
			paragraph: { language: 'Cobol', rule: { kind: 'paragraph' } },
		},
		kinds: {},
	},
} satisfies Configuration;

export const nestedRuleTypo = {
	...configuration,
	evidence: {
		kind: 'inline',
		matchers: {
			// expect TS2561
			paragraph: { language: 'Markdown', rule: { kindz: 'paragraph' } },
		},
		kinds: {},
	},
} satisfies Configuration;

export const numberForFullWidthLimit = {
	...configuration,
	// expect TS2322
	limits: { max_file_bytes: 1048576 },
} satisfies Configuration;

export const strictCheckedFilesKey = {
	...configuration,
	// expect TS2353
	check: { files: ['**/*.md'], include: ['x'] },
} satisfies Configuration;

export const wrongScope = run(configuration, {
	root: '/abs/root',
	// expect TS2322
	scope: { kind: 'everything' },
});

export const missingRoot = run(
	configuration,
	// expect TS2741
	{ scope: { kind: 'all' } },
);

export function wrongOutcomeKind(outcome: RunOutcome): boolean {
	// expect TS2367
	return outcome.kind === 'complete';
}

export function reportWithoutNarrowing(outcome: RunOutcome): number {
	// expect TS2339
	return outcome.report.findings.length;
}

export function authoredSpellingIsNotAResultLanguage(language: EvidenceLanguage): boolean {
	// expect TS2367
	return language === 'TypeScript';
}

export function loadedValuesAreReadonly(loaded: LoadedConfiguration): void {
	// expect TS2540
	loaded.configuration.id = 'changed';
}

export async function inventedResultMember(): Promise<unknown> {
	const outcome = await check(configuration);
	if (outcome.kind === 'valid') {
		// expect TS2339
		return outcome.summary.rule_total;
	}
	return load({ directory: '/abs' });
}

// HTML section selectors take one filter: starts_with or pattern, never both.
export const doublyFilteredSections = {
	html_units: {
		mode: 'unit',
		units: ['section'],
		starts_with: 'Parent',
		// expect TS2322
		pattern: 'Child',
	},
	fields: {},
} satisfies EvidenceKindDefinition;

export const unknownSectionText = {
	html_units: { mode: 'unit', units: ['section'] },
	// expect TS2322
	fields: { body: { from: 'section', text: 'parent' } },
} satisfies EvidenceKindDefinition;

// Upstream message and severity have defaults; explicit null is not a value.
export const nullMatcherMessage = {
	language: 'Markdown',
	rule: { kind: 'paragraph' },
	// expect TS2322
	message: null,
} satisfies AstMatcher;

export const nullMatcherSeverity = {
	language: 'Markdown',
	rule: { kind: 'paragraph' },
	// expect TS2322
	severity: null,
} satisfies AstMatcher;

// Present-only fields may be omitted but never null.
export const nullKindRule = {
	// expect TS2322
	rule: null,
	fields: {},
} satisfies EvidenceKindDefinition;

export const nullKindHtmlUnits = {
	// expect TS2322
	html_units: null,
	fields: {},
} satisfies EvidenceKindDefinition;

export const nullKindWithin = {
	rule: 'paragraph',
	fields: {},
	// expect TS2322
	within: null,
} satisfies EvidenceKindDefinition;

export const nullOnlyWhen = {
	name: 'gated',
	evidence: 'paragraph',
	// expect TS2322
	only_when: null,
	decision: {
		start: 'done',
		steps: {
			done: {
				mechanical_check: { tool: 'complete', field: 'body' },
				yes: 'pass',
				no: { finding: 'Empty.' },
			},
		},
	},
} satisfies RuleEntry;

export const nullNoulCriteria = {
	per: 'skills/{name}',
	reads: { file: 'SKILL.md', kind: 'paragraph', fields: ['body'] },
	labels: ['yes'],
	questions: {
		// expect TS2322
		asked: { type: 'noul', instructions: 'Is it?', criteria: null },
	},
	decision: {
		start: 'asked',
		steps: {
			asked: {
				ask: 'asked',
				yes_at_or_above: 0.8,
				no_at_or_below: 0.2,
				yes: { label: 'yes' },
				no: { label: 'yes' },
				uncertain: 'unsure',
			},
		},
	},
} satisfies FilterDefinition;

export const capturedNullWithin = {
	...configuration,
	evidence: {
		kind: 'captured',
		matchers: {},
		utilities: {},
		// expect TS2322
		kinds: { paragraph: { rule: 'paragraph', fields: {}, within: null } },
	},
} satisfies Configuration;

// A selection question is a noul judgment question, not arbitrary JSON.
export const primitiveSelectionQuestion = {
	rule: 'paragraph',
	fields: {},
	select: {
		// expect TS2322
		question: false,
		yes_at_or_above: 0.8,
		no_at_or_below: 0.2,
	},
} satisfies EvidenceKindDefinition;

export const choiceSelectionQuestion = {
	rule: 'paragraph',
	fields: {},
	select: {
		// expect TS2322
		question: { type: 'choice', instructions: 'Which?' },
		yes_at_or_above: 0.8,
		no_at_or_below: 0.2,
	},
} satisfies EvidenceKindDefinition;

export const misspelledSelectionQuestion = {
	rule: 'paragraph',
	fields: {},
	select: {
		// expect TS2561
		question: { type: 'noul', instruction: 'Is it?' },
		yes_at_or_above: 0.8,
		no_at_or_below: 0.2,
	},
} satisfies EvidenceKindDefinition;

// A kind's Markdown selection is unit mode only, with known units and one filter.
export const markdownCallSite = {
	markdown_units: {
		// expect TS2322
		mode: 'call_site',
		units: ['section'],
		starts_with: 'Steps',
	},
	fields: {},
} satisfies EvidenceKindDefinition;

export const markdownUnknownUnit = {
	// expect TS2322
	markdown_units: { mode: 'unit', units: ['heading'] },
	fields: {},
} satisfies EvidenceKindDefinition;

export const markdownNull = {
	// expect TS2322
	markdown_units: null,
	fields: {},
} satisfies EvidenceKindDefinition;

export const markdownTwoFilters = {
	markdown_units: {
		mode: 'unit',
		units: ['section'],
		starts_with: 'A',
		// expect TS2322
		pattern: 'B',
	},
	fields: {},
} satisfies EvidenceKindDefinition;

// An evidence kind names exactly one matcher: not none, and not two.
export const noMatcherKind = {
	fields: { body: { from: 'remainder' } },
	// expect TS1360
} satisfies EvidenceKindDefinition;

export const twoMatcherKind = {
	rule: 'paragraph',
	inline_rule: 'kind: paragraph',
	fields: { body: { from: 'remainder' } },
	// expect TS1360
} satisfies EvidenceKindDefinition;

// A kind passed by reference, without a fresh literal's excess-property check.
const noMatcherValue = { fields: {} };
// expect TS2322
export const noMatcherByReference: EvidenceKindDefinition = noMatcherValue;

const twoMatcherValue = { rule: 'paragraph', inline_rule: 'kind: paragraph', fields: {} };
// expect TS2322
export const twoMatcherByReference: EvidenceKindDefinition = twoMatcherValue;

// Document conventions: known keys, the closed scheme and the closed unsupported reasons.
export const misspelledHtmlKey = {
	// expect TS2561
	skipTags: ['ex-markdown'],
} satisfies HtmlEvidenceSettings;

export const unknownMarkdownKey = {
	// expect TS2353
	scheme: 'github',
} satisfies MarkdownEvidenceSettings;

export const unknownScheme = {
	// expect TS2322
	heading_anchors: 'gitlab',
} satisfies MarkdownEvidenceSettings;

export const nonStringTag = {
	// expect TS2322
	skip_tags: [1],
} satisfies HtmlEvidenceSettings;

// expect TS2322
export const unknownReason: TargetAnchorUnsupportedReason = 'not_configured';

// expect TS2322
export const unsupportedWithoutReason: TargetAnchor = { status: 'unsupported', fragment: 'top' };

export const reasonOnFound = {
	status: 'found',
	fragment: 'top',
	// expect TS2353
	reason: 'unsupported_document',
} satisfies TargetAnchor;

// Heading text is a closed literal, only on section and context sources.
export const misspelledSectionText = {
	from: 'section',
	// expect TS2820
	text: 'headline',
} satisfies FieldSource;

export const misspelledContextText = {
	from: 'enclosing',
	kind: 'outline',
	// expect TS2820
	text: 'headings',
} satisfies FieldSource;

export const headingOnFirstLine = {
	from: 'first_line',
	// expect TS2322
	text: 'heading',
} satisfies FieldSource;

export const sectionWithoutText = {
	from: 'section',
	// expect TS1360
} satisfies FieldSource;

// Codec roots are tagged and must carry the value matching the tag.
// expect TS2345
export const documentMissingValue = serialize({ kind: 'configuration' });
// expect TS2322
export const documentUnknownKind = serialize({ kind: 'unknown', value: configuration });
// expect TS2322
export const documentWrongValue = serialize({ kind: 'configuration', value: 'text' });
// expect TS2345
export const deserializeWrongInput = deserialize(123);
// expect TS2741
export const inspectionMissingRoot = inspect(configuration, { scope: { kind: 'all' } });

export async function inspectionWithoutNarrowing(): Promise<number> {
	const outcome = await inspect(configuration, { root: '/abs', scope: { kind: 'all' } });
	// expect TS2339
	return outcome.report.pieces.length;
}

export async function decodedWithoutNarrowing(): Promise<unknown> {
	const outcome = await deserialize('{}');
	// expect TS2339
	return outcome.value;
}
