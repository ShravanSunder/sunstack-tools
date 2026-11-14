import type { CountBound, DecisionMechanicalTool, SectionScope } from '../../src/index.ts';

export const ownScope: SectionScope = { section_own: ['html_outline', 'markdown_outline'] };

export const bounds: readonly CountBound[] = [
	{ at_least: 100n },
	{ at_most: 20n },
	{ greater_than: 7n },
];
export const checks: readonly DecisionMechanicalTool[] = [
	{ tool: 'word_count', field: ['html_prose', 'markdown_prose'], bound: { at_least: 100n } },
	{ tool: 'piece_count', field: 'rows', bound: { greater_than: 20n } },
];
