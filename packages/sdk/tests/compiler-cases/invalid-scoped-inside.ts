import type { FieldSource, SectionScope } from '../../src/index.ts';

export const ruleScope: FieldSource = {
	from: 'inside',
	rule: 'image',
	// expect TS2322
	scope: { section: 'outline' },
};
// expect TS2322
export const scopeWithoutKind: FieldSource = { from: 'inside', scope: { section: 'outline' } };
// expect TS2322
export const nullScope: FieldSource = { from: 'inside', kind: 'picture', scope: null };
// expect TS2322
export const bothSectionKeys: SectionScope = { section: 'outline', section_prefix: 'outline' };
// expect TS2353
export const unknownScopeKey: SectionScope = { heading: 'outline' };
export const invalidVariantMember: FieldSource = {
	from: 'inside',
	kind: 'picture',
	scope: { section: 'outline' },
	// expect TS2322
	text: 'full',
};
// expect TS2322
export const ruleAndKindScope: FieldSource = {
	from: 'inside',
	kind: 'picture',
	rule: 'image',
	scope: { section: 'outline' },
};

const ruleScopeValue = { from: 'inside', rule: 'image', scope: { section: 'outline' } } as const;
// expect TS2322
export const ruleScopeByReference: FieldSource = ruleScopeValue;
const nullScopeValue = { from: 'inside', kind: 'picture', scope: null } as const;
// expect TS2322
export const nullScopeByReference: FieldSource = nullScopeValue;
