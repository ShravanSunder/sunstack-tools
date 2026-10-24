import { textAnchorMatches, type CommentAnchor, type TextCommentAnchor } from './review-model.ts';

export interface ReviewTarget {
	readonly targetId: string;
	readonly label: string;
	readonly element: HTMLElement;
}
export type ResolvedCommentAnchor =
	| { readonly kind: 'resolved'; readonly target: ReviewTarget; readonly range: Range | null }
	| { readonly kind: 'unavailable'; readonly message: string };

const excludedText =
	'button,input,textarea,select,script,style,template,.ex-inspect-controls,[data-comment-control],[data-inspector-copy],[data-inspector-content]';

function fragmentText(fragment: DocumentFragment): string {
	for (const control of fragment.querySelectorAll(excludedText)) control.remove();
	return fragment.textContent ?? '';
}

export function reviewTextNodes(element: HTMLElement): Text[] {
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

export function reviewTargetText(element: HTMLElement): string {
	return reviewTextNodes(element)
		.map((node): string => node.data)
		.join('');
}

export function readReviewTargets(root: HTMLElement): Map<string, ReviewTarget> {
	const targets = new Map<string, ReviewTarget>();
	const candidates = [
		root,
		...root.querySelectorAll<HTMLElement>(
			'ex-choice[id],.ex-choice-option[id],[data-comment-label][id]',
		),
	];
	for (const element of candidates) {
		if (!element.id || element.closest('[data-inspector-copy],[data-inspector-content]')) continue;
		if (targets.has(element.id)) throw new Error(`Duplicate review target id: ${element.id}`);
		const label =
			[
				element.getAttribute('data-comment-label'),
				element.getAttribute('title'),
				element.querySelector('h1')?.textContent,
			]
				.find((value): value is string => Boolean(value?.trim()))
				?.trim() ?? element.id;
		targets.set(element.id, { targetId: element.id, label, element });
	}
	return targets;
}

function offsetBefore(element: HTMLElement, container: Node, offset: number): number {
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
	const ancestor =
		range.commonAncestorContainer instanceof Element
			? range.commonAncestorContainer
			: range.commonAncestorContainer.parentElement;
	if (ancestor?.closest(excludedText)) return undefined;
	let element = ancestor instanceof HTMLElement ? ancestor : ancestor?.parentElement;
	while (element && !targets.has(element.id)) element = element.parentElement;
	const startElement =
		range.startContainer instanceof Element
			? range.startContainer
			: range.startContainer.parentElement;
	const endElement =
		range.endContainer instanceof Element ? range.endContainer : range.endContainer.parentElement;
	const blockSelector = 'p,li,blockquote,pre,h1,h2,h3,h4,h5,h6';
	const crossesBlocks = startElement?.closest(blockSelector) !== endElement?.closest(blockSelector);
	const target = targets.get(crossesBlocks ? root.id : (element?.id ?? root.id));
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
