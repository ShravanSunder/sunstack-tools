import type { EvidenceKindDefinition, FieldValue } from '../../src/index.ts';

export const kind = {
	rule: 'subject',
	fields: {
		ids: { from: 'captures', regex: '\\bU[0-9]+\\b' },
		rows: {
			from: 'document',
			path: ['specification.md'],
			rule: 'row',
			containing: 'ids',
			attach: { until: 'boundary', max_siblings: 12 },
		},
		names: { from: 'captures', of: 'rows', regex: '`([A-Z][A-Za-z]+)`' },
		local: { from: 'same_file', rule: 'row', attach: { until: 'boundary', max_siblings: 2 } },
	},
} satisfies EvidenceKindDefinition;
export const partial = {
	state: 'texts',
	values: ['AttemptOutcome'],
	spans: [],
	cut: true,
} satisfies FieldValue;
