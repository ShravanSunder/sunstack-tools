import type { FieldSource } from '../../src/index.ts';

// expect TS2322
export const nullInput = { from: 'captures', regex: 'U', of: null } satisfies FieldSource;
// expect TS2322
export const listInput = { from: 'captures', regex: 'U', of: ['body'] } satisfies FieldSource;
export const nullAttachment = {
	from: 'document',
	path: 'x',
	rule: 'r',
	// expect TS2322
	attach: null,
} satisfies FieldSource;
export const missingCap = {
	from: 'same_file',
	rule: 'r',
	// expect TS2741
	attach: { until: 'boundary' },
} satisfies FieldSource;
