import type typescript from 'typescript';

import type { DesignRecord } from '../schema.ts';
import {
	rewriteReference,
	type AddressMaps,
	type AnchorRelocations,
	type ReferenceOwner,
} from './address-rewrite.ts';
import {
	matchesPath,
	replaceSite,
	stringSites,
	type LiteralPath,
	type TextEdit,
} from './source-edits.ts';

/**
 * Which strings in a record, deliverable or design literal are references, links or
 * ids, and how each changes when its item moves.
 */
export const helperForKind: Readonly<Record<DesignRecord['kind'], string>> = {
	need: 'defineNeed',
	rule: 'defineRule',
	realization: 'defineRealization',
	'planned-check': 'defineCheck',
	choice: 'defineChoice',
	ruling: 'defineRuling',
	assumption: 'defineAssumption',
};
/** A placeholder for a moved record's helper call, bound to the target file's import. */
export const helperToken = (helper: string): string => `__recut_helper_${helper}__`;
export const helperTokenPattern = /__recut_helper_(define[A-Za-z]+)__/gu;

interface LiteralRule {
	readonly pattern: readonly string[];
	readonly role: 'key' | 'value';
	readonly action: 'record' | 'deliverable' | 'href' | 'id' | 'destination' | 'fragment';
}

export const recordRules: readonly LiteralRule[] = [
	...['governedBy', 'reliesOn'].map((field): LiteralRule => ({
		pattern: [field, '*'],
		role: 'value',
		action: 'record',
	})),
	...['serves', 'realizes', 'checks'].flatMap((field): LiteralRule[] => [
		{ pattern: [field], role: 'value', action: 'record' },
		{ pattern: [field, '*'], role: 'value', action: 'record' },
	]),
	{ pattern: ['supersedes'], role: 'value', action: 'record' },
	{ pattern: ['id'], role: 'value', action: 'id' },
	{ pattern: ['sources', '*', 'href'], role: 'value', action: 'href' },
	{ pattern: ['authority', 'source', 'href'], role: 'value', action: 'href' },
	{ pattern: ['resolution', 'href'], role: 'value', action: 'href' },
];

export const deliverableRules: readonly LiteralRule[] = [
	{ pattern: ['id'], role: 'value', action: 'id' },
	...['needs', 'rules', 'designItems', 'verificationChecks'].map((field): LiteralRule => ({
		pattern: ['scope', field, '*'],
		role: 'value',
		action: 'record',
	})),
	{ pattern: ['verification', 'outcomes', '*'], role: 'key', action: 'record' },
	{ pattern: ['verificationHistory', '*', 'outcomes', '*'], role: 'key', action: 'record' },
	{ pattern: ['implementation', 'evidence', '*'], role: 'value', action: 'href' },
	{ pattern: ['verification', 'outcomes', '*', 'evidence'], role: 'value', action: 'href' },
	{
		pattern: ['verificationHistory', '*', 'outcomes', '*', 'evidence'],
		role: 'value',
		action: 'href',
	},
	{ pattern: ['destination', 'page'], role: 'value', action: 'destination' },
	{ pattern: ['destination', 'fragment'], role: 'value', action: 'fragment' },
];

export const designRules: readonly LiteralRule[] = [
	{ pattern: ['mapGroups', '**', 'target', 'record'], role: 'value', action: 'record' },
	{
		pattern: ['walkthroughs', '*', 'steps', '*', 'target', 'record'],
		role: 'value',
		action: 'record',
	},
	{ pattern: ['acceptances', '*', 'deliverableId'], role: 'value', action: 'deliverable' },
	{ pattern: ['acceptances', '*', 'evidence'], role: 'value', action: 'href' },
];

export const acceptanceRules: readonly LiteralRule[] = [
	{ pattern: ['deliverableId'], role: 'value', action: 'deliverable' },
	{ pattern: ['evidence'], role: 'value', action: 'href' },
];

export interface LiteralContext {
	readonly owner: ReferenceOwner;
	/** The file links are read from, before and after the run. */
	readonly linkBase: { readonly before: string; readonly after: string };
	readonly newId?: string;
	readonly destinationPage?: string;
	readonly destinationFragment?: string;
}

export interface LiteralRewrites {
	readonly maps: AddressMaps;
	readonly anchors: AnchorRelocations;
}

export function literalEdits(
	root: typescript.Expression,
	source: typescript.SourceFile,
	rules: readonly LiteralRule[],
	context: LiteralContext,
	inputs: LiteralRewrites,
	skip: (path: LiteralPath) => boolean = (): boolean => false,
): TextEdit[] {
	const edits: TextEdit[] = [];
	for (const site of stringSites(root, source)) {
		if (skip(site.path)) continue;
		const rule = rules.find(
			(candidate) => candidate.role === site.role && matchesPath(site.path, candidate.pattern),
		);
		if (!rule) continue;
		const value = ((): string => {
			switch (rule.action) {
				case 'record':
				case 'deliverable':
					return rewriteReference(site.value, rule.action, context.owner, inputs.maps);
				case 'href':
					return inputs.anchors.relocate(
						site.value,
						context.linkBase.before,
						context.linkBase.after,
					);
				case 'id':
					return context.newId ?? site.value;
				case 'destination':
					return context.destinationPage ?? site.value;
				case 'fragment':
					return context.destinationFragment ?? site.value;
				default:
					return rule.action satisfies never;
			}
		})();
		const edit = replaceSite(site, value);
		if (edit) edits.push(edit);
	}
	return edits;
}

/** design.ts paths the record and page passes own. */
export function recordOrPagePath(path: LiteralPath): boolean {
	return path[0] === 'records' || path[0] === 'pages';
}
