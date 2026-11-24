import { parse, type DefaultTreeAdapterMap } from 'parse5';

import { designRecordId } from '../../src/core/decision-record.ts';
import { migratedOptionAnchor } from '../../src/core/design-record-view.ts';
import { escapeHtml } from '../../src/core/html-text.ts';
import { migrateOptionDetails } from './record-authorship.ts';

interface ReleasedOption {
	readonly id: string;
	readonly title: string;
}

type PageNode = DefaultTreeAdapterMap['node'];
type PageElement = DefaultTreeAdapterMap['element'];

function attribute(element: PageElement, name: string): string | undefined {
	return element.attrs.find((entry) => entry.name === name)?.value;
}

export function migrateReleasedChoices(
	file: string,
	html: string,
): {
	readonly output: string;
	readonly choices: readonly { reference: string; options: readonly ReleasedOption[] }[];
} {
	const edits: { start: number; end: number; replacement: string }[] = [];
	const choices: { reference: string; options: ReleasedOption[] }[] = [];
	function visit(node: PageNode): void {
		if ('tagName' in node && node.tagName === 'ex-choice') {
			const reference = attribute(node, 'data-design-record');
			if (reference !== undefined) {
				const options: ReleasedOption[] = [];
				const taken = new Set<string>();
				const declarations = node.childNodes.filter(
					(child): child is PageElement => 'tagName' in child && child.tagName === 'ex-option',
				);
				const reserved = new Set(declarations.map((child) => attribute(child, 'id')));
				for (const [index, child] of declarations.entries()) {
					const anchor = attribute(child, 'id');
					const title = attribute(child, 'title');
					const locations = child.sourceCodeLocation?.attrs;
					const idRange = locations?.['id'];
					const titleRange = locations?.['title'];
					if (!anchor || /\s/u.test(anchor) || !title?.trim() || !idRange || !titleRange)
						throw new Error(
							`${file}: ${reference} option ${index + 1} needs its released ID and title.`,
						);
					let optionId = anchor;
					if (!designRecordId.safeParse(optionId).success) {
						optionId = `option-${index + 1}`;
						while (reserved.has(optionId) || taken.has(optionId)) optionId += '-legacy';
					}
					if (taken.has(optionId))
						throw new Error(`${file}: ${reference} has duplicate option ${anchor}.`);
					taken.add(optionId);
					options.push({ id: optionId, title });
					edits.push(
						{
							start: idRange.startOffset,
							end: idRange.endOffset,
							replacement: `data-option="${escapeHtml(optionId)}" ${migratedOptionAnchor}="${escapeHtml(anchor)}"`,
						},
						{ start: titleRange.startOffset, end: titleRange.endOffset, replacement: '' },
					);
				}
				if (options.length < 2)
					throw new Error(
						`${file}: ${reference} has fewer than two released options. Supply the missing page option.`,
					);
				choices.push({ reference, options });
			}
		}
		if ('childNodes' in node) node.childNodes.forEach(visit);
	}
	visit(parse(html, { sourceCodeLocationInfo: true }));
	let output = html;
	for (const { start, end, replacement } of edits.toSorted(
		(left, right) => right.start - left.start,
	))
		output = output.slice(0, start) + replacement + output.slice(end);
	if (!choices.length) return { output, choices };
	try {
		return { output: migrateOptionDetails(output), choices };
	} catch (error) {
		throw new Error(
			`${file}: ${choices.map(({ reference }) => reference).join(', ')}: ${error instanceof Error ? error.message : String(error)}`,
			{ cause: error },
		);
	}
}
