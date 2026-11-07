import type { DefaultTreeAdapterMap } from 'parse5';

import { storySlots } from '../story-vocabulary.ts';

type AuthoredElement = DefaultTreeAdapterMap['element'];

export interface StoryMarkupProblem {
	readonly element: AuthoredElement;
	readonly rule: 'story' | 'section';
	readonly message: string;
}

function childrenOf(element: AuthoredElement): AuthoredElement[] {
	return element.childNodes.filter((child): child is AuthoredElement => 'tagName' in child);
}

function attribute(element: AuthoredElement, name: string): string | undefined {
	return element.attrs.find((candidate): boolean => candidate.name === name)?.value;
}

/** Validate authored structure before enhancement, using the same named-child contract as runtime. */
export function checkStoryMarkup(elements: readonly AuthoredElement[]): StoryMarkupProblem[] {
	const problems: StoryMarkupProblem[] = [];
	for (const element of elements) {
		if (element.tagName === 'ex-section') {
			const children = childrenOf(element);
			const smaller = children.filter((child): boolean => attribute(child, 'slot') === 'smaller');
			if (
				smaller.length > 1 ||
				(smaller[0] && (smaller[0].tagName !== 'details' || children.at(-1) !== smaller[0]))
			)
				problems.push({
					element,
					rule: 'section',
					message: 'A section has at most one native smaller-stories fold, at its end.',
				});
		}
		if (element.tagName !== 'ex-story') continue;
		const roots = childrenOf(element);
		const root = roots[0];
		if (roots.length !== 1 || !root || root.tagName !== 'details') {
			problems.push({
				element,
				rule: 'story',
				message: 'A story needs exactly one direct native details root.',
			});
			continue;
		}
		const content = childrenOf(root);
		if (
			!attribute(root, 'id')?.trim() ||
			content.filter((child): boolean => child.tagName === 'summary').length !== 1
		)
			problems.push({
				element: root,
				rule: 'story',
				message: 'A story root needs a stable id and one native summary.',
			});
		for (const slot of storySlots) {
			const matches = content.filter((child): boolean => attribute(child, 'slot') === slot.name);
			if (matches.length > 1 || (slot.required && matches.length !== 1))
				problems.push({
					element: root,
					rule: 'story',
					message: `A story needs ${slot.required ? 'exactly' : 'at most'} one ${slot.name} child.`,
				});
			for (const child of matches) {
				if (
					slot.disclosure &&
					(child.tagName !== 'details' ||
						childrenOf(child).filter((node): boolean => node.tagName === 'summary').length !== 1)
				)
					problems.push({
						element: child,
						rule: 'story',
						message: `The ${slot.name} child needs native details and one summary.`,
					});
				if (slot.name === 'record')
					for (const entry of childrenOf(child).filter(
						(node): boolean => node.tagName === 'details',
					))
						if (
							!attribute(entry, 'id')?.trim() ||
							childrenOf(entry).filter((node): boolean => node.tagName === 'summary').length !== 1
						)
							problems.push({
								element: entry,
								rule: 'story',
								message: 'Each record entry needs its supplied id and one native summary.',
							});
			}
		}
		for (const child of content) {
			if (child.tagName === 'summary') continue;
			const slot = attribute(child, 'slot');
			if (!storySlots.some((candidate): boolean => candidate.name === slot))
				problems.push({
					element: child,
					rule: 'story',
					message: 'Story body content needs a supported named slot.',
				});
		}
	}
	return problems;
}
