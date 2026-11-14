import type { HtmlEvidenceSettings, HtmlMarkdownIsland } from '../../src/index.ts';

// expect TS2741
export const missingTag: HtmlMarkdownIsland = {};
export const unknownMember = {
	tag: 'script',
	// expect TS2353
	selector: 'text/markdown',
} satisfies HtmlMarkdownIsland;
export const nullSelectors = {
	// expect TS2322
	embedded_markdown: null,
} satisfies HtmlEvidenceSettings;
export const nullAttributes = {
	tag: 'script',
	// expect TS2322
	attributes: null,
} satisfies HtmlMarkdownIsland;
export const numberAttribute = {
	tag: 'script',
	// expect TS2322
	attributes: { type: 1 },
} satisfies HtmlMarkdownIsland;
export const nullSelector = {
	// expect TS2322
	embedded_markdown: [null],
} satisfies HtmlEvidenceSettings;
