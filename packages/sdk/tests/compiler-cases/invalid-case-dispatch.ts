import type { CaseBinding, CaseExecutionOwner, RunOptions } from '../../src/index.ts';
import { inspect } from '../../src/index.ts';
import { configuration } from './valid-caller.ts';
import { byteOwner, filterCase, ruleCase } from './valid-case-dispatch.ts';
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

export const numberBinding = {
	...ruleCase,
	// expect TS2322
	bytes: [0, 12n],
} satisfies CaseBinding;
export const stringBinding = {
	...ruleCase,
	// expect TS2322
	bytes: ['0', 12n],
} satisfies CaseBinding;
export const shortBinding = {
	...ruleCase,
	// expect TS2322
	bytes: [0n],
} satisfies CaseBinding;
export const longBinding = {
	...ruleCase,
	// expect TS2322
	bytes: [0n, 12n, 24n],
} satisfies CaseBinding;
export const numberOwner = {
	...byteOwner,
	// expect TS2322
	bytes: [0n, 12],
} satisfies CaseExecutionOwner;
export const stringOwner = {
	...byteOwner,
	// expect TS2322
	bytes: [0n, '12'],
} satisfies CaseExecutionOwner;
export const shortOwner = {
	...byteOwner,
	// expect TS2322
	bytes: [0n],
} satisfies CaseExecutionOwner;
export const longOwner = {
	...byteOwner,
	// expect TS2322
	bytes: [0n, 12n, 24n],
} satisfies CaseExecutionOwner;
// expect TS2322
export const missingOwnerBytes: CaseExecutionOwner = {
	kind: 'rule',
	rule: 'body',
	file: 'docs/one.md',
	lines: [3n, 3n],
};

export function mutateBindingBytes(binding: CaseBinding): void {
	if (binding.kind === 'rule' && binding.bytes !== undefined && binding.bytes !== null) {
		// expect TS2540
		binding.bytes[0] = 1n;
		// expect TS2540
		binding.bytes = [0n, 12n];
	}
}
export function mutateOwnerBytes(owner: CaseExecutionOwner): void {
	if (owner.kind === 'rule') {
		// expect TS2540
		owner.bytes[1] = 12n;
		// expect TS2540
		owner.bytes = [0n, 12n];
	}
}
