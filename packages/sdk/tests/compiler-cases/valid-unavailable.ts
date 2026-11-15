import type {
	CaseBindingStatus,
	RulePieceEnding,
	RulePieceResult,
	RunOutcome,
	UnavailableCounts,
	WorkUnavailableReason,
} from '../../src/index.ts';

export const reasons = [
	{ kind: 'credential_unavailable' },
	{ kind: 'filter_unavailable', filter: 'purpose', folder: 'skills/guide' },
	{ kind: 'selection_unavailable', evidence_kind: 'paragraph', file: 'guide.md' },
	{ kind: 'selected_field_unavailable', field: 'body' },
] as const satisfies readonly WorkUnavailableReason[];

export const unavailableEnding = {
	outcome: 'unavailable',
	reason: reasons[0],
} as const satisfies RulePieceEnding;

export const counts = {
	rule_pieces: 1n,
	filter_folders: 2n,
	selection_candidates: 3n,
	evidence_scopes: 4n,
} as const satisfies UnavailableCounts;

export function unavailableReason(reason: WorkUnavailableReason): string {
	switch (reason.kind) {
		case 'credential_unavailable':
			return reason.kind;
		case 'filter_unavailable':
			return `${reason.filter}/${reason.folder}`;
		case 'selection_unavailable':
			return `${reason.evidence_kind}/${reason.file}`;
		case 'selected_field_unavailable':
			return reason.field;
		default:
			return assertNever(reason);
	}
}

export function readUnavailableResult(result: RulePieceResult): string {
	if (result.outcome === 'unavailable') {
		const identity: bigint = result.id;
		const line: bigint = result.first_line;
		return `${identity}/${line}/${unavailableReason(result.reason)}`;
	}
	return result.outcome;
}

export function readRun(outcome: RunOutcome): bigint | string {
	if (outcome.kind === 'incomplete') return outcome.report.unavailable_counts.rule_pieces;
	if (outcome.kind === 'unavailable')
		return outcome.diagnostics.map((entry) => entry.code).join(',');
	return outcome.kind;
}

export function readCase(status: CaseBindingStatus): string {
	return status.kind === 'unavailable' ? unavailableReason(status.reason) : status.kind;
}

function assertNever(value: never): never {
	throw new Error(String(value));
}
