import type {
	CaseBinding,
	CaseExecutionOwner,
	CaseSubjectBinding,
	CaseBindingStatus,
	CaseSelectionReport,
	RunOptions,
} from '../../src/index.ts';
import { run } from '../../src/index.ts';
import { configuration } from './valid-caller.ts';

export const ruleCase = {
	id: 'rule-case',
	kind: 'rule',
	rule: 'body',
	file: 'docs/one.md',
	lines: [3n, 3n],
	text_sha256: 'a'.repeat(64),
} as const satisfies CaseBinding;
export const filterCase = {
	id: 'filter-case',
	kind: 'filter',
	filter: 'purpose',
	folder: 'skills/alpha',
	reads_sha256: 'b'.repeat(64),
} as const satisfies CaseBinding;
export const selected = {
	root: '/abs/root',
	scope: { kind: 'all' },
	case_selection: [ruleCase, filterCase],
} as const satisfies RunOptions;
export const selectedRun = run(configuration, selected);
export const emptyRun = run(configuration, {
	root: '/abs/root',
	scope: { kind: 'all' },
	case_selection: [],
});
export function selectedIds(report: CaseSelectionReport): readonly string[] {
	return report.bindings.map((result) => result.binding.id);
}
export function statusKind(status: CaseBindingStatus): string {
	switch (status.kind) {
		case 'selected':
			return status.owner.kind;
		case 'ambiguous':
			return String(status.owners.length);
		case 'selection_pending':
			return status.file;
		case 'incomplete':
			return JSON.stringify(status.reason);
		case 'unavailable':
			return status.reason.kind;
		case 'unmatched':
		case 'stale':
		case 'cancelled':
			return status.kind;
		default:
			return assertNever(status);
	}
}
function assertNever(value: never): never {
	throw new Error(String(value));
}

export const ordinaryPaths = ['docs/one.md', 'docs/two.md'];
export const ordinaryArrayRun = run(configuration, {
	root: '/abs/root',
	scope: { kind: 'all' },
	paths: ordinaryPaths,
});

export const exactBytes = [9007199254740993n, 9007199254740994n] as const;
export const byteBinding = { ...ruleCase, bytes: exactBytes } as const satisfies CaseBinding;
export const byteSubject: CaseSubjectBinding = byteBinding;
export const legacyBinding: CaseBinding = ruleCase;
export const byteOwner = {
	kind: 'rule',
	rule: 'body',
	file: 'docs/one.md',
	lines: [3n, 3n],
	bytes: exactBytes,
} as const satisfies CaseExecutionOwner;
export const byteOptions = {
	root: '/abs/root',
	scope: { kind: 'all' },
	case_selection: [byteBinding, legacyBinding],
} as const satisfies RunOptions;

export function ownerBytes(owner: CaseExecutionOwner): readonly [bigint, bigint] | undefined {
	return owner.kind === 'rule' ? owner.bytes : undefined;
}
export function bindingBytes(binding: CaseBinding): readonly [bigint, bigint] | null | undefined {
	return binding.kind === 'rule' ? binding.bytes : undefined;
}
