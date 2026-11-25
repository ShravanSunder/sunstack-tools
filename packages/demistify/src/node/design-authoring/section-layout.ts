import type { DefaultTreeAdapterMap } from 'parse5';

import { isDecision } from '../../core/decision-record.ts';
import { type SectionHold, sectionHolds } from '../../core/section-routing.ts';
import { qualifyRecord, type DesignRecord } from './schema.ts';

type Element = DefaultTreeAdapterMap['element'];

export interface SectionLayout {
	readonly sections: readonly {
		readonly id: string;
		readonly holds?: SectionHold;
		readonly line: number;
	}[];
	readonly items: readonly {
		readonly record?: string;
		readonly section?: string;
		readonly line: number;
	}[];
}

const emptySectionMessage =
	'An ex-section must be empty. Put its content in an ex-section-item outside the section.';

function attribute(element: Element, name: string): string | undefined {
	const value = element.attrs.find((entry) => entry.name === name)?.value.trim();
	if (value === '') return undefined;
	return value;
}

function inside(element: Element, tag: string): boolean {
	for (
		let parent = element.parentNode;
		parent;
		parent = 'parentNode' in parent ? parent.parentNode : null
	)
		if ('tagName' in parent && parent.tagName === tag) return true;
	return false;
}

export function sectionMarkupProblem(element: Element): string | undefined {
	if (
		element.tagName === 'ex-section' &&
		element.childNodes.some(
			(child) => 'tagName' in child || ('value' in child && !!child.value.trim()),
		)
	)
		return emptySectionMessage;
	if (element.tagName !== 'ex-section-item') return undefined;
	if (inside(element, 'ex-section'))
		return 'Write ex-section-item outside ex-section. Name the section with section="id" or route it with record="ID".';
	if (inside(element, 'ex-section-item'))
		return 'An ex-section-item cannot contain another ex-section-item. Put the items beside each other in source order.';
	if (Boolean(attribute(element, 'record')) === Boolean(attribute(element, 'section')))
		return 'An ex-section-item needs exactly one destination. Use record="ID" or section="id".';
	return undefined;
}

export function readSectionLayout(elements: readonly Element[], file: string): SectionLayout {
	const sections: SectionLayout['sections'][number][] = [];
	const items: SectionLayout['items'][number][] = [];
	for (const element of elements) {
		const line = element.sourceCodeLocation?.startLine ?? 1;
		if (element.tagName === 'ex-section') {
			const id = attribute(element, 'id');
			const holds = attribute(element, 'holds');
			const value = sectionHolds.find((candidate) => candidate === holds);
			if (!id)
				throw new Error(
					`${file}:${line}: Give the ex-section an id so items and links can name it.`,
				);
			if (holds && !value)
				throw new Error(
					`${file}:${line}: Unknown holds value "${holds}". Use ${sectionHolds.join(', ')}.`,
				);
			if (value && sections.some((section) => section.holds === value))
				throw new Error(
					`${file}:${line}: holds="${value}" already has a section. Keep one section per holds value.`,
				);
			sections.push({ id, line, ...(value && { holds: value }) });
		} else if (element.tagName === 'ex-section-item') {
			const record = attribute(element, 'record');
			const section = attribute(element, 'section');
			items.push({ line, ...(record && { record }), ...(section && { section }) });
		}
	}
	for (const { section, line } of items) {
		if (!section) continue;
		if (!sections.some(({ id, holds }) => id === section && !holds))
			throw new Error(
				`${file}:${line}: Item names unknown plain section "${section}". Declare <ex-section id="${section}" title="…"></ex-section> or use an existing section id.`,
			);
	}
	return { sections, items };
}

export function requiredSectionHolds(record: DesignRecord): readonly SectionHold[] {
	if (record.kind === 'assumption') return ['assumed'];
	if (!isDecision(record)) return [];
	if (record.authority.state === 'owner-open')
		return ['needs-owner', 'settled', 'decided-by-agent'];
	if (record.authority.state === 'agent-decided') return ['decided-by-agent', 'settled'];
	return ['settled'];
}

export function projectSectionItems(
	layout: SectionLayout,
	records: ReadonlyMap<string, DesignRecord>,
	file: string,
	namespace: string | undefined,
	placed: Set<string>,
): { reference: string; holds: SectionHold }[] {
	const result: { reference: string; holds: SectionHold }[] = [];
	for (const { record: reference, line } of layout.items) {
		if (!reference) continue;
		const qualified = qualifyRecord(namespace, reference);
		const record = records.get(qualified);
		if (!record || (!isDecision(record) && record.kind !== 'assumption'))
			throw new Error(
				`${file}:${line}: Item names unknown decision or assumption "${reference}". Use an existing record ID or define it.`,
			);
		if (placed.has(qualified))
			throw new Error(
				`${file}:${line}: Record "${reference}" already has an ex-section-item. Keep its explanation in one item.`,
			);
		placed.add(qualified);
		const destinations = requiredSectionHolds(record);
		for (const holds of destinations) {
			if (!layout.sections.some((section) => section.holds === holds))
				throw new Error(
					`${file}:${line}: Record "${reference}" needs a section with holds="${holds}". A record moves between sections as it is decided, so its page declares every section it can reach: ${destinations.join(', ')}. Declare <ex-section id="…" title="…" holds="${holds}"></ex-section> on this page.`,
				);
		}
		const holds = destinations[0];
		if (holds) result.push({ reference, holds });
	}
	return result;
}
