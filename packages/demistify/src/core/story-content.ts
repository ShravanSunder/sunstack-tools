import { storySlots } from '../story-vocabulary.ts';
import { createPhosphorIcon } from './icons.ts';

export function storyRoot(owner: Element): HTMLDetailsElement | undefined {
	const root = owner.querySelector(':scope > details');
	return root instanceof HTMLDetailsElement ? root : undefined;
}

export function storySlot(owner: Element, name: string): Element | undefined {
	const root = storyRoot(owner);
	const body = root?.querySelector(':scope > .ex-story-body') ?? root;
	return Array.from(body?.children ?? []).find(
		(child): boolean => child.getAttribute('slot') === name,
	);
}

/** Read native disclosure membership after wrappers are prepared, without copying its open state. */
export function storyDisclosures(owner: Element): HTMLDetailsElement[] {
	const root = storyRoot(owner);
	if (!root) return [];
	const members = [root];
	for (const name of ['more', 'record']) {
		const slot = storySlot(owner, name);
		if (!(slot instanceof HTMLDetailsElement)) continue;
		members.push(slot);
		if (name === 'record')
			for (const child of slot.children)
				if (child instanceof HTMLDetailsElement) members.push(child);
	}
	return members;
}

export function storyContentProblems(owner: HTMLElement): string[] {
	const roots = Array.from(owner.children);
	const root = storyRoot(owner);
	if (roots.length !== 1 || !root) return ['A story needs exactly one direct native details root.'];
	const content = Array.from(root.children);
	const problems: string[] = [];
	if (!root.id || content.filter((child): boolean => child.localName === 'summary').length !== 1)
		problems.push('A story root needs a stable id and one native summary.');
	for (const slot of storySlots) {
		const matches = content.filter((child): boolean => child.getAttribute('slot') === slot.name);
		if (matches.length > 1 || (slot.required && matches.length !== 1))
			problems.push(
				`A story needs ${slot.required ? 'exactly' : 'at most'} one ${slot.name} child.`,
			);
		for (const child of matches) {
			if (
				slot.disclosure &&
				(!(child instanceof HTMLDetailsElement) ||
					child.querySelectorAll(':scope > summary').length !== 1)
			)
				problems.push(`The ${slot.name} child needs native details and one summary.`);
			if (slot.name === 'record')
				for (const entry of child.children)
					if (
						entry instanceof HTMLDetailsElement &&
						(!entry.id || entry.querySelectorAll(':scope > summary').length !== 1)
					)
						problems.push('Each record entry needs its supplied id and one native summary.');
		}
	}
	for (const child of content)
		if (
			child.localName !== 'summary' &&
			!storySlots.some((slot): boolean => slot.name === child.getAttribute('slot'))
		)
			problems.push('Story body content needs a supported named slot.');
	return problems;
}

/** Decorate the existing native summary and retain all authored inline nodes and disclosure state. */
export function prepareStoryDisclosure(
	details: HTMLDetailsElement,
	icon: Node | undefined,
	metadata: string | undefined,
): void {
	const summary = details.querySelector(':scope > summary');
	if (!(summary instanceof HTMLElement) || details.classList.contains('ex-story-disclosure'))
		return;
	details.classList.add('ex-story-disclosure');
	summary.addEventListener('click', (event: MouseEvent): void => {
		const selection = summary.ownerDocument.getSelection();
		// A drag selects the account's title; Enter/Space and ordinary clicks still toggle it.
		if (
			event.detail > 0 &&
			selection &&
			!selection.isCollapsed &&
			selection.containsNode(summary, true)
		)
			event.preventDefault();
	});
	const label = document.createElement('span');
	label.className = 'ex-story-title';
	label.append(...Array.from(summary.childNodes));
	const chevron = createPhosphorIcon('caret-right');
	chevron.classList.add('ex-story-chevron');
	label.append(chevron);
	if (metadata) {
		const note = document.createElement('small');
		note.className = 'ex-story-meta';
		note.textContent = metadata;
		label.append(note);
	}
	if (icon) summary.append(icon);
	summary.append(label);
}
