import type { FieldSource, FieldValue } from '../../src/index.ts';

// expect TS2322
export const numericPath: FieldSource = { from: 'document', path: 42, rule: 'document' };
// expect TS2322
export const numericCandidate: FieldSource = { from: 'document', path: [42], rule: 'document' };
// expect TS2322
export const missingCandidates: FieldValue = { state: 'missing_document' };
// expect TS2322
export const missingChosenPath: FieldValue = { state: 'document', values: [], spans: [] };
// expect TS2322
export const missingValues: FieldValue = { state: 'document', path: 'verification.md', spans: [] };
// expect TS2322
export const wrongCandidates: FieldValue = { state: 'missing_document', candidates: [42] };
export const nullDocumentFilter: FieldSource = {
	from: 'document',
	path: 'verification.md',
	rule: 'row',
	// expect TS2322
	containing: null,
};
// expect TS2322
export const nullSameFileFilter: FieldSource = { from: 'same_file', rule: 'row', containing: null };
