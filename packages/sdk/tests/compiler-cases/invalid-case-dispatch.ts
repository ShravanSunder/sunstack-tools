import type { CaseBinding, RunOptions } from '../../src/index.ts';
import { inspect } from '../../src/index.ts';
import { configuration } from './valid-caller.ts';
import { filterCase, ruleCase } from './valid-case-dispatch.ts';
export const wrongWidth = {
	...ruleCase,
	// expect TS2322
	lines: [3, 3n],
} satisfies CaseBinding;
export const leakedLabel = {
	...ruleCase,
	// expect TS2353
	expected: 'pass',
} satisfies CaseBinding;
// expect TS2322
export const missingDigest: CaseBinding = {
	id: 'missing',
	kind: 'filter',
	filter: 'purpose',
	folder: 'skills/alpha',
};
export const unknownKind = {
	...ruleCase,
	// expect TS2322
	kind: 'piece',
} satisfies CaseBinding;
export const wrongSelection = {
	root: '/abs',
	scope: { kind: 'all' },
	// expect TS2322
	case_selection: 'rule-case',
} satisfies RunOptions;
export const inspectionSelection = inspect(configuration, {
	root: '/abs',
	scope: { kind: 'all' },
	// expect TS2353
	case_selection: [ruleCase],
});

export const shortRange = {
	...ruleCase,
	// expect TS2322
	lines: [3n],
} satisfies CaseBinding;
export const longRange = {
	...ruleCase,
	// expect TS2322
	lines: [3n, 3n, 3n],
} satisfies CaseBinding;

export const leakedFilterLabel = {
	...filterCase,
	// expect TS2353
	expected_label: 'eligible',
} satisfies CaseBinding;
