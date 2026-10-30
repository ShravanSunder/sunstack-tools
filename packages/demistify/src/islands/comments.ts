import { createCommentStore, type CommentStore } from '../core/comment-store.ts';
import { createCommentIcon } from '../core/icons.ts';
import { reportIssue } from '../core/issues.ts';
import {
	captureCommentAnchor,
	readReviewTargets,
	reviewTextNodes,
	resolveCommentAnchor,
	type ReviewTarget,
} from '../core/review-anchors.ts';
import { resolveReviewDocumentIdentity } from '../core/review-document.ts';
import {
	reviewEnvelopeSchema,
	type CommentAnchor,
	type PageComment,
	type ReviewEnvelope,
} from '../core/review-model.ts';
import { revealElement } from '../layout/target-resolution.ts';
import { readChoiceResponses } from './choice.ts';
import { CommentsView } from './comments-view.ts';

const boundDocuments = new WeakMap<HTMLElement, CommentsIsland>();

/** One original document owns feedback; inspector copies never bind another controller. */
export class CommentsIsland extends HTMLElement {
	private root: HTMLElement | undefined;
	private store: CommentStore | undefined;
	private view: CommentsView | undefined;
	private trigger: HTMLButtonElement | undefined;
	private selectionButton: HTMLButtonElement | undefined;
	private targets = new Map<string, ReviewTarget>();
	private controls = new Map<Element, HTMLButtonElement>();
	private events = new AbortController();
	private observer: MutationObserver | undefined;
	private frame: number | undefined;
	private candidate: ReturnType<typeof captureCommentAnchor>;
	private composing: CommentAnchor | undefined;
	private active: PageComment | undefined;
	private readonly highlightName = `ex-comments-${crypto.randomUUID()}`;
	private readonly highlightStyle = document.createElement('style');
	private connectionGeneration = 0;

	get review(): ReviewEnvelope {
		if (!this.store || !this.root) throw new Error('Comments is not bound to a document.');
		return reviewEnvelopeSchema.parse({
			...this.store.snapshot(),
			answers: readChoiceResponses(this.root),
		});
	}

	connectedCallback(): void {
		if (this.closest('[data-inspector-copy],[data-inspector-content]')) return;
		const generation = ++this.connectionGeneration;
		void this.connect(generation).catch((error: unknown): void => {
			if (!this.isConnected || generation !== this.connectionGeneration) return;
			const message = document.createElement('p');
			message.setAttribute('role', 'alert');
			message.textContent = `Comments could not load: ${String(error)}`;
			this.append(message);
			reportIssue(this, message.textContent);
		});
	}

	private async connect(generation: number): Promise<void> {
		const binding = this.getAttribute('for')?.trim();
		const root = binding ? document.getElementById(binding) : null;
		if (
			!(root instanceof HTMLElement) ||
			root.localName !== 'ex-document' ||
			root.closest('[data-inspector-copy],[data-inspector-content]')
		)
			throw new Error('ex-comments needs for="…" naming an original ex-document.');
		const identity = await resolveReviewDocumentIdentity(root);
		if (!this.isConnected || generation !== this.connectionGeneration) return;
		if (boundDocuments.has(root) && boundDocuments.get(root) !== this)
			throw new Error('A document can have only one Comments surface.');
		const existing = this.store?.snapshot();
		if (
			existing &&
			(existing.documentId !== identity.documentId ||
				existing.documentRevision !== identity.documentRevision)
		)
			throw new Error(
				'A Comments instance cannot change document identity or revision. Use a new Comments surface.',
			);
		this.root = root;
		this.store ??= createCommentStore(identity);
		boundDocuments.set(root, this);
		this.events.abort();
		this.events = new AbortController();
		const { signal } = this.events;
		this.trigger ??= this.createTrigger();
		this.selectionButton ??= this.createSelectionButton();
		document.body.append(this.trigger, this.selectionButton);
		this.highlightStyle.textContent = `::highlight(${this.highlightName}){background:var(--annotation-wash);text-decoration:underline;text-decoration-color:var(--annotation-color)}::highlight(${this.highlightName}-active){background:color-mix(in srgb,var(--annotation-color) 40%,transparent);text-decoration:underline;text-decoration-color:var(--annotation-color)}`;
		this.append(this.highlightStyle);
		this.view ??= new CommentsView({
			review: (): ReviewEnvelope => this.review,
			issue: (): string | undefined => this.storageIssue(),
			save: (anchor, body, commentId): void => {
				this.store?.save(anchor, body, commentId);
				this.changed();
			},
			remove: (commentId): void => {
				this.store?.remove(commentId);
				if (this.active?.commentId === commentId) this.active = undefined;
				this.changed();
			},
			reveal: (comment): string | undefined => this.reveal(comment),
			composing: (anchor): void => {
				this.composing = anchor;
				if (this.selectionButton) this.selectionButton.hidden = true;
				this.repaint();
			},
			visibility: (open): void => {
				this.trigger?.setAttribute('aria-expanded', String(open));
			},
		});
		this.trigger.addEventListener('click', (): void => this.view?.toggle(), { signal });
		this.selectionButton.addEventListener('pointerdown', (event): void => event.preventDefault(), {
			signal,
		});
		this.selectionButton.addEventListener(
			'click',
			(): void => {
				if (this.candidate) this.view?.compose(this.candidate.anchor);
			},
			{ signal },
		);
		document.addEventListener('selectionchange', (): void => this.captureSelection(), { signal });
		document.addEventListener(
			'keydown',
			(event: KeyboardEvent): void => {
				if (
					event.defaultPrevented ||
					event.isComposing ||
					event.repeat ||
					event.metaKey ||
					event.ctrlKey ||
					event.altKey ||
					event.key.toLowerCase() !== 'c' ||
					(event.target instanceof Element &&
						event.target.closest(
							'input,textarea,select,[contenteditable]:not([contenteditable="false"]),[role="textbox"]',
						))
				)
					return;
				event.preventDefault();
				this.view?.toggle();
			},
			{ signal },
		);
		root.addEventListener('island-ready', (): void => this.refreshTargets(), { signal });
		root.addEventListener(
			'choice-change',
			(): void => {
				this.view?.refresh();
				this.changed();
			},
			{ signal },
		);
		this.observer = new MutationObserver((records): void => {
			if (
				!records.some(
					(record): boolean =>
						!(
							record.target instanceof Element &&
							record.target.closest('[data-comment-control],.ex-inspect-controls')
						) &&
						(record.type !== 'childList' ||
							[...record.addedNodes, ...record.removedNodes].some(
								(node): boolean =>
									!(node instanceof Element && node.matches('[data-comment-control]')),
							)),
				)
			)
				return;
			this.frame ??= requestAnimationFrame((): void => {
				this.frame = undefined;
				this.refreshTargets();
			});
		});
		this.observer.observe(root, {
			subtree: true,
			childList: true,
			characterData: true,
			attributes: true,
			attributeFilter: ['id', 'data-comment-label'],
		});
		this.refreshTargets();
		this.view.refresh();
	}

	disconnectedCallback(): void {
		this.connectionGeneration++;
		this.events.abort();
		this.observer?.disconnect();
		if (this.frame !== undefined) cancelAnimationFrame(this.frame);
		this.frame = undefined;
		this.view?.close();
		this.trigger?.remove();
		this.selectionButton?.remove();
		this.highlightStyle.remove();
		for (const [element, control] of this.controls) {
			control.remove();
			element.classList.remove('ex-comment-active', 'ex-comment-target');
		}
		this.controls.clear();
		if (this.root && boundDocuments.get(this.root) === this) boundDocuments.delete(this.root);
		if (typeof CSS.highlights !== 'undefined') {
			CSS.highlights.delete(this.highlightName);
			CSS.highlights.delete(`${this.highlightName}-active`);
		}
	}

	private createTrigger(): HTMLButtonElement {
		const button = document.createElement('button');
		button.type = 'button';
		button.className = 'ex-icon-button ex-comments-trigger';
		button.title = 'Show or hide comments (C)';
		button.setAttribute('aria-keyshortcuts', 'C');
		button.setAttribute('aria-expanded', 'false');
		button.setAttribute('aria-label', 'Comments (0)');
		return button;
	}
	private createSelectionButton(): HTMLButtonElement {
		const button = document.createElement('button');
		button.type = 'button';
		button.className = 'ex-selection-comment';
		button.append(createCommentIcon(), 'Comment');
		button.setAttribute('aria-label', 'Comment on selected text');
		button.hidden = true;
		return button;
	}
	private storageIssue(): string | undefined {
		const state = this.store?.persistence();
		return state && state.kind !== 'ready' ? state.message : undefined;
	}
	private changed(): void {
		this.repaint();
		this.dispatchEvent(
			new CustomEvent<ReviewEnvelope>('review-change', {
				bubbles: true,
				composed: true,
				detail: this.review,
			}),
		);
	}

	private refreshTargets(): void {
		if (!this.root) return;
		try {
			this.targets = readReviewTargets(this.root);
		} catch (error) {
			reportIssue(this, String(error));
			return;
		}
		const originalNodes = new Set(
			[...this.targets.values()].map((target): Element => target.element),
		);
		for (const [element, control] of this.controls)
			if (!originalNodes.has(element)) {
				control.remove();
				element.classList.remove('ex-comment-target');
				this.controls.delete(element);
			}
		for (const target of this.targets.values()) {
			// Table selections have local targets without inserting controls into table structure.
			if (
				target.element.matches('table,tr,td,th') &&
				!target.element.matches('.ex-choice-option,[data-comment-label]')
			)
				continue;
			// SVG nodes are selection targets; HTML buttons cannot be inserted into SVG.
			if (!(target.element instanceof HTMLElement)) continue;
			// The page already has its shared Comments launcher. Keep the root as the
			// cross-block anchor without stacking a second icon over its first heading.
			if (target.element === this.root && !this.root.hasAttribute('data-comment-label')) continue;
			target.element.classList.add('ex-comment-target');
			const existing = this.controls.get(target.element);
			if (existing?.isConnected) continue;
			existing?.remove();
			const button = document.createElement('button');
			button.type = 'button';
			button.className = 'ex-entity-comment';
			button.dataset['commentControl'] = '';
			button.title = `Comment on ${target.label}`;
			button.addEventListener('pointerdown', (event): void => event.preventDefault());
			button.addEventListener('click', (): void => {
				const selected = this.candidate;
				const anchor: CommentAnchor =
					selected && target.element.contains(selected.range.commonAncestorContainer)
						? selected.anchor
						: { kind: 'entity', targetId: target.targetId, label: target.label };
				button.focus({ preventScroll: true });
				this.view?.compose(anchor);
			});
			(target.element.querySelector(':scope > [data-comment-slot]') ?? target.element).append(
				button,
			);
			this.controls.set(target.element, button);
		}
		this.repaint();
	}

	private captureSelection(): void {
		if (!this.root || !this.selectionButton) return;
		const selection = window.getSelection();
		if (selection?.anchorNode && !this.root.contains(selection.anchorNode)) {
			this.candidate = undefined;
			this.selectionButton.hidden = true;
			return;
		}
		this.candidate = selection
			? captureCommentAnchor(selection, this.targets, this.root)
			: undefined;
		this.selectionButton.hidden = !this.candidate;
		if (!this.candidate) return;
		const bounds = this.candidate.range.getBoundingClientRect();
		this.selectionButton.style.left = `${Math.max(8, Math.min(bounds.left, innerWidth - 140))}px`;
		this.selectionButton.style.top = `${Math.max(52, Math.min(bounds.bottom + 8, innerHeight - 52))}px`;
	}

	private reveal(comment: PageComment): string | undefined {
		const target = this.targets.get(comment.anchor.targetId);
		if (target) revealElement(target.element);
		const resolved = resolveCommentAnchor(comment.anchor, this.targets);
		if (resolved.kind === 'unavailable') return resolved.message;
		this.active = comment;
		const range = resolved.range;
		if (range) {
			const passageNodes = reviewTextNodes(resolved.target.element).filter(
				(node): boolean =>
					range.intersectsNode(node) &&
					Boolean(
						node.data
							.slice(
								node === range.startContainer ? range.startOffset : 0,
								node === range.endContainer ? range.endOffset : node.length,
							)
							.trim(),
					),
			);
			for (const node of passageNodes) if (node.parentElement) revealElement(node.parentElement);
			const passage = passageNodes[0]?.parentElement;
			// The phone sheet closes after this handler; measure the restored reader on its next frame.
			requestAnimationFrame((): void => {
				if (!this.isConnected || this.active !== comment || !passage?.isConnected) return;
				passage.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'instant' });
				const bounds = range.getBoundingClientRect();
				const view = passage.ownerDocument.defaultView;
				view?.scrollBy({
					top: bounds.top - (view.innerHeight - Math.min(bounds.height, view.innerHeight - 96)) / 2,
					behavior: 'smooth',
				});
			});
		} else resolved.target.element.scrollIntoView({ block: 'center', behavior: 'smooth' });
		this.repaint();
		return undefined;
	}

	private repaint(): void {
		if (!this.store || !this.trigger) return;
		const snapshot = this.store.snapshot();
		const badge = document.createElement('span');
		badge.className = 'ex-comments-count';
		badge.textContent = String(snapshot.comments.length);
		badge.hidden = !snapshot.comments.length;
		this.trigger.replaceChildren(createCommentIcon(snapshot.comments.length > 0), badge);
		this.trigger.setAttribute('aria-label', `Comments (${snapshot.comments.length})`);
		for (const target of this.targets.values()) {
			const count = snapshot.comments.filter(
				(comment): boolean => comment.anchor.targetId === target.targetId,
			).length;
			const button = this.controls.get(target.element);
			button?.replaceChildren(createCommentIcon(count > 0));
			if (button) {
				button.dataset['commentCount'] = String(count);
				button.setAttribute(
					'aria-label',
					`Comment on ${target.label}${count ? ` (${count})` : ''}`,
				);
			}
			target.element.classList.toggle(
				'ex-comment-active',
				this.active?.anchor.targetId === target.targetId,
			);
		}
		if (typeof Highlight === 'undefined' || !CSS.highlights) return;
		const saved: Range[] = [];
		const active: Range[] = [];
		for (const comment of snapshot.comments) {
			const resolved = resolveCommentAnchor(comment.anchor, this.targets);
			if (resolved.kind === 'resolved' && resolved.range)
				(comment.commentId === this.active?.commentId ? active : saved).push(resolved.range);
		}
		if (this.composing) {
			const resolved = resolveCommentAnchor(this.composing, this.targets);
			if (resolved.kind === 'resolved' && resolved.range) active.push(resolved.range);
		}
		CSS.highlights.set(this.highlightName, new Highlight(...saved));
		CSS.highlights.set(`${this.highlightName}-active`, new Highlight(...active));
	}
}
