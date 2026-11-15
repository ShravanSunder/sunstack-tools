import type {
	RulePieceEnding,
	RulePieceResult,
	RunOutcome,
	UnavailableCounts,
	WorkUnavailableReason,
} from '../../src/index.ts';

// expect TS2322
export const missingReason: RulePieceEnding = { outcome: 'unavailable' };
// expect TS2322
export const missingFilterFolder: WorkUnavailableReason = {
	kind: 'filter_unavailable',
	filter: 'purpose',
};
// expect TS2322
export const missingSelectionFile: WorkUnavailableReason = {
	kind: 'selection_unavailable',
	evidence_kind: 'paragraph',
};
// expect TS2322
export const missingSelectedField: WorkUnavailableReason = { kind: 'selected_field_unavailable' };
// expect TS2322
export const unknownReason: WorkUnavailableReason = { kind: 'credential_missing' };

export function countAsNumber(counts: UnavailableCounts): number {
	// expect TS2322
	return counts.rule_pieces;
}

export function mutateCounts(counts: UnavailableCounts): void {
	// expect TS2540
	counts.evidence_scopes = 0n;
}

export function reasonBeforeNarrowing(result: RulePieceResult): unknown {
	// expect TS2339
	return result.reason;
}

export function mutateReason(result: RulePieceResult): void {
	if (result.outcome === 'unavailable') {
		// expect TS2540
		result.reason = { kind: 'credential_unavailable' };
	}
}

export function reportOnCaptureUnavailable(outcome: RunOutcome): unknown {
	if (outcome.kind === 'unavailable') {
		// expect TS2339
		return outcome.report;
	}
	return outcome.kind;
}
