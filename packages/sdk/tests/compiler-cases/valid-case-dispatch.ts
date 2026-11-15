import type {
	CaseBinding,
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
