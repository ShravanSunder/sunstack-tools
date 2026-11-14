import type { FieldSource, SectionScope } from '../../src/index.ts';

export const unscopedRule: FieldSource = { from: 'inside', rule: 'image' };
export const unscopedKind: FieldSource = { from: 'inside', kind: 'picture' };
export const prefix: FieldSource = {
	from: 'inside',
	kind: 'picture',
	scope: { section_prefix: 'outline' },
};
export const full: FieldSource = { from: 'inside', kind: 'picture', scope: { section: 'outline' } };
export const prefixScope: SectionScope = { section_prefix: 'outline' };
export const sectionScope: SectionScope = { section: 'outline' };
