import type { FieldSource, FieldValue } from '../../src/index.ts';

export const scalar: FieldSource = { from: 'document', path: 'verification.md', rule: 'document' };
export const candidates: FieldSource = {
	from: 'document',
	path: ['verification.md', 'verification-design.md'],
	rule: 'document',
};
export const scopedDocument: FieldSource = {
	from: 'document',
	path: ['verification.html', 'verification.md'],
	rule: 'row',
	containing: 'identifier',
};
export const scopedSameFile: FieldSource = {
	from: 'same_file',
	rule: 'row',
	text: 'full',
	containing: 'requirement_ids',
};
export const selected: FieldValue = {
	state: 'document',
	path: 'verification-design.md',
	values: ['U1 covered.'],
	spans: [],
};
export const missing: FieldValue = {
	state: 'missing_document',
	candidates: ['verification.md', 'verification-design.md'],
};

export function chosenPath(field: FieldValue): string | undefined {
	return field.state === 'document' ? field.path : undefined;
}
