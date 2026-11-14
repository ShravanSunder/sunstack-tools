import type { EvidenceKindDefinition } from '../../src/index.ts';

export const html: EvidenceKindDefinition = {
	html_units: { mode: 'unit', units: ['section'], levels: [1, 2] },
	fields: {},
};
export const markdown: EvidenceKindDefinition = {
	markdown_units: { mode: 'unit', units: ['section'], levels: [2], starts_with: 'Decision' },
	fields: {},
};
