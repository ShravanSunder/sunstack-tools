// Each marked line must fail with exactly the named diagnostic; nothing else may fail.
import { check, load, run } from '../../src/index.ts';
import type {
	Configuration,
	EvidenceKindDefinition,
	EvidenceLanguage,
	LoadedConfiguration,
	RunOutcome,
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
