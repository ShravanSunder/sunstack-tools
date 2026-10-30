import { textAnchorMatches, type CommentAnchor, type TextCommentAnchor } from './review-model.ts';

export interface ReviewTarget {
	readonly targetId: string;
	readonly label: string;
	readonly element: Element;
}
export type ResolvedCommentAnchor =
	| { readonly kind: 'resolved'; readonly target: ReviewTarget; readonly range: Range | null }
	| { readonly kind: 'unavailable'; readonly message: string };

const excludedText =
	'button,input,textarea,select,script,style,template,.ex-inspect-controls,.ex-choice-inline > summary,[data-comment-control],[data-inspector-copy],[data-inspector-content]';
const automaticallyLabeledHeadings = new WeakSet<HTMLElement>();

function fragmentText(fragment: DocumentFragment): string {
	for (const control of fragment.querySelectorAll(excludedText)) control.remove();
	return fragment.textContent ?? '';
}

export function reviewTextNodes(element: Element): Text[] {
	const walker = element.ownerDocument.createTreeWalker(element, NodeFilter.SHOW_TEXT, {
		acceptNode(node): number {
			return node.parentElement?.closest(excludedText)
				? NodeFilter.FILTER_REJECT
				: NodeFilter.FILTER_ACCEPT;
		},
	});
	const nodes: Text[] = [];
	let node = walker.nextNode();
	while (node) {
		if (node instanceof Text) nodes.push(node);
		node = walker.nextNode();
	}
	return nodes;
}

export function reviewTargetText(element: Element): string {
	return reviewTextNodes(element)
		.map((node): string => node.data)
		.join('');
}

/** Structural identity is stable within the same authored content revision. */
export function ensureReviewTargetId(element: Element, root: HTMLElement): void {
	if (element.id) return;
	const path: string[] = [];
	let current: Element | null = element;
	while (current && current !== root && !current.id) {
		const parent: Element | null = current.parentElement;
		if (!parent) break;
		const siblings = [...parent.children].filter(
			(sibling): boolean => sibling.localName === current?.localName,
		);
		path.unshift(`${current.localName}-${siblings.indexOf(current) + 1}`);
		current = parent;
	}
	const base = `${current?.id || root.id}--review-${path.join('-')}`;
	let candidate = base;
	let suffix = 1;
	while (root.ownerDocument.getElementById(candidate)) candidate = `${base}-${++suffix}`;
	element.id = candidate;
}

export function readReviewTargets(root: HTMLElement): Map<string, ReviewTarget> {
	const targets = new Map<string, ReviewTarget>();
	// Existing descendants get their historical IDs before newly targetable table ancestors do.
	const candidates = new Set([
		root,
		...root.querySelectorAll(
			'ex-choice[id],.ex-choice-option[id],[data-comment-label],p,li,h1,h2,h3,h4,h5,h6,ex-callout,ex-card,ex-tree,ex-file-tree,ex-code,ex-mermaid,ex-pre,ex-mermaid g.node[data-id]',
		),
		...root.querySelectorAll('table,tr,td,th'),
	]);
	for (const element of candidates) {
		if (element.closest(`${excludedText},ex-comments,nav,[role="tablist"]`)) continue;
		const generatedOwner = element.parentElement?.closest(
			'ex-choice,ex-tree,ex-file-tree,ex-explorer,ex-gallery,ex-stat',
		);
		if (generatedOwner && !element.hasAttribute('data-comment-label')) continue;
		if (element.matches('ex-mermaid g.node[data-id]')) {
			const owner = element.closest('ex-mermaid');
			if (owner) {
				const scopedNodeId = `${owner.id}--${element.getAttribute('data-id') ?? ''}`;
				if (element.id !== scopedNodeId) element.id = scopedNodeId;
			}
		}
		ensureReviewTargetId(element, root);
		if (
			element instanceof HTMLElement &&
			element.matches('h1,h2,h3,h4,h5,h6') &&
			(!element.hasAttribute('aria-label') || automaticallyLabeledHeadings.has(element))
		) {
			// An interactive descendant otherwise changes the heading's accessible name.
			element.setAttribute('aria-label', reviewTargetText(element).replace(/\s+/g, ' ').trim());
			automaticallyLabeledHeadings.add(element);
		}
		if (targets.has(element.id)) throw new Error(`Duplicate review target id: ${element.id}`);
		const label =
			[
				element.getAttribute('data-comment-label'),
				element.getAttribute('title'),
				element.getAttribute('label'),
				element.querySelector('h1')?.textContent,
				reviewTargetText(element).replace(/\s+/g, ' ').trim().slice(0, 120),
			]
				.find((value): value is string => Boolean(value?.trim()))
				?.trim() ?? element.id;
		targets.set(element.id, { targetId: element.id, label, element });
	}
	return targets;
}

function offsetBefore(element: Element, container: Node, offset: number): number {
	const prefix = element.ownerDocument.createRange();
	prefix.selectNodeContents(element);
	prefix.setEnd(container, offset);
	return fragmentText(prefix.cloneContents()).length;
}

/** A cross-block selection falls back to the original document target. */
export function captureCommentAnchor(
	selection: Selection,
	targets: ReadonlyMap<string, ReviewTarget>,
	root: HTMLElement,
): { readonly anchor: TextCommentAnchor; readonly range: Range } | undefined {
	if (!selection.rangeCount || selection.isCollapsed || !selection.toString().trim())
		return undefined;
	const range = selection.getRangeAt(0).cloneRange();
	if (!root.contains(range.startContainer) || !root.contains(range.endContainer)) return undefined;
	// A browser range may include whitespace siblings outside the visible highlighted paragraph.
	// Choose its owner from the selected readable text, keeping genuine cross-block ranges intact.
	const selectedNodes = reviewTextNodes(root).filter((node): boolean => {
		if (!range.intersectsNode(node)) return false;
		return Boolean(
			node.data
				.slice(
					node === range.startContainer ? range.startOffset : 0,
					node === range.endContainer ? range.endOffset : node.length,
				)
				.trim(),
		);
	});
	const firstNode = selectedNodes[0];
	const lastNode = selectedNodes.at(-1);
	if (!firstNode || !lastNode) return undefined;
	if (range.startContainer !== firstNode) range.setStart(firstNode, 0);
	if (range.endContainer !== lastNode) range.setEnd(lastNode, lastNode.length);
	const ancestor =
		range.commonAncestorContainer instanceof Element
			? range.commonAncestorContainer
			: range.commonAncestorContainer.parentElement;
	if (ancestor?.closest(excludedText)) return undefined;
	let element: Element | null | undefined = ancestor;
	while (element && !targets.has(element.id)) element = element.parentElement;
	const startElement =
		range.startContainer instanceof Element
			? range.startContainer
			: range.startContainer.parentElement;
	const endElement =
		range.endContainer instanceof Element ? range.endContainer : range.endContainer.parentElement;
	const blockSelector = 'p,li,blockquote,pre,h1,h2,h3,h4,h5,h6';
	const crossesBlocks = startElement?.closest(blockSelector) !== endElement?.closest(blockSelector);
	const explicitOwner = ancestor?.closest(
		'[data-comment-label][id],.ex-choice-option[id],ex-choice[id],ex-code[id],ex-pre[id],ex-mermaid g.node[id],ex-mermaid[id]',
	);
	const startChoice = startElement?.closest('.ex-choice-option[id],ex-choice[id]');
	const endChoice = endElement?.closest('.ex-choice-option[id],ex-choice[id]');
	const choiceOwner = startChoice === endChoice ? startChoice : undefined;
	const componentOwner = ancestor?.closest(
		'td[id],th[id],tr[id],table[id],ex-code[id],ex-pre[id],ex-mermaid[id]',
	);
	const componentTarget = componentOwner ? targets.get(componentOwner.id) : undefined;
	const explicitTarget = explicitOwner ? targets.get(explicitOwner.id) : undefined;
	const target = targets.get(
		crossesBlocks || startChoice !== endChoice
			? (choiceOwner?.id ?? componentTarget?.targetId ?? root.id)
			: (explicitTarget?.targetId ?? element?.id ?? root.id),
	);
	if (!target) return undefined;
	const text = reviewTargetText(target.element);
	const start = offsetBefore(target.element, range.startContainer, range.startOffset);
	const end = offsetBefore(target.element, range.endContainer, range.endOffset);
	const sourceText = text.slice(start, end);
	if (!sourceText.trim()) return undefined;
	return {
		anchor: {
			kind: 'text',
			targetId: target.targetId,
			label: target.label,
			selectedText: selection.toString(),
			sourceText,
			start,
			end,
			prefix: text.slice(Math.max(0, start - 32), start),
			suffix: text.slice(end, end + 32),
		},
		range,
	};
}

export function resolveCommentAnchor(
	anchor: CommentAnchor,
	targets: ReadonlyMap<string, ReviewTarget>,
): ResolvedCommentAnchor {
	const target = targets.get(anchor.targetId);
	if (!target?.element.isConnected)
		return { kind: 'unavailable', message: 'The original item is not on this page.' };
	if (anchor.kind === 'entity') return { kind: 'resolved', target, range: null };
	if (!textAnchorMatches(reviewTargetText(target.element), anchor))
		return { kind: 'unavailable', message: 'The selected text or its context has changed.' };
	const range = target.element.ownerDocument.createRange();
	let count = 0;
	let started = false;
	for (const node of reviewTextNodes(target.element)) {
		const end = count + node.data.length;
		if (!started && anchor.start <= end) {
			range.setStart(node, anchor.start - count);
			started = true;
		}
		if (started && anchor.end <= end) {
			range.setEnd(node, anchor.end - count);
			return { kind: 'resolved', target, range };
		}
		count = end;
	}
	return { kind: 'unavailable', message: 'The original text range could not be restored.' };
}
