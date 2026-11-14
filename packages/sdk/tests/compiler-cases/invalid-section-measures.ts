import type { CountBound, DecisionMechanicalTool, SectionScope } from '../../src/index.ts';

// expect TS2322
export const mixed: CountBound = { at_least: 1n, at_most: 2n };
// expect TS2322
export const missing: CountBound = {};
// expect TS2322
export const rounded: CountBound = { at_least: 100 };
// expect TS2322
export const missingField: DecisionMechanicalTool = { tool: 'piece_count', bound: { at_most: 1n } };
// expect TS2322
export const mixedScope: SectionScope = { section_own: ['outline'], section: 'outline' };
