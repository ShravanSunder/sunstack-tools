import type { HtmlEvidenceSettings, HtmlMarkdownIsland } from '../../src/index.ts';

export const tagOnly = { tag: 'custom-md' } satisfies HtmlMarkdownIsland;
export const attributed = {
	tag: 'script',
	attributes: { type: 'text/markdown', 'data-purpose': 'guide' },
} satisfies HtmlMarkdownIsland;
export const configured = {
	embedded_markdown: [tagOnly, attributed],
} satisfies HtmlEvidenceSettings;
export const omitted = {} satisfies HtmlEvidenceSettings;
export const empty = { embedded_markdown: [] } satisfies HtmlEvidenceSettings;
