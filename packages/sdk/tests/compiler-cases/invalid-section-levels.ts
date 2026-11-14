import type { EvidenceKindDefinition } from '../../src/index.ts';

export const nullHtml: EvidenceKindDefinition = {
	html_units: {
		mode: 'unit',
		units: ['section'],
		// expect TS2322
		levels: null,
	},
	fields: {},
};
export const stringHtml: EvidenceKindDefinition = {
	html_units: {
		mode: 'unit',
		units: ['section'],
		// expect TS2322
		levels: ['2'],
	},
	fields: {},
};
export const nullMarkdown: EvidenceKindDefinition = {
	markdown_units: {
		mode: 'unit',
		units: ['section'],
		// expect TS2322
		levels: null,
	},
	fields: {},
};
export const stringMarkdown: EvidenceKindDefinition = {
	markdown_units: {
		mode: 'unit',
		units: ['section'],
		// expect TS2322
		levels: ['2'],
	},
	fields: {},
};
