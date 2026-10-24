import { createIconButton, createPhosphorIcon } from '../core/icons.ts';
import {
	formatReview,
	type CommentAnchor,
	type PageComment,
	type ReviewEnvelope,
} from '../core/review-model.ts';
import { openDrawer } from './drawer.ts';

interface CommentsViewOptions {
	readonly review: () => ReviewEnvelope;
	readonly issue: () => string | undefined;
	readonly save: (anchor: CommentAnchor, body: string, commentId?: string) => void;
	readonly reveal: (comment: PageComment) => string | undefined;
	readonly composing: (anchor: CommentAnchor | undefined) => void;
	readonly visibility: (open: boolean) => void;
}

function textElement<TTag extends keyof HTMLElementTagNameMap>(
	tag: TTag,
	text: string,
	className = '',
): HTMLElementTagNameMap[TTag] {
	const element = document.createElement(tag);
	element.textContent = text;
	element.className = className;
	return element;
}

/** Comments borrows the same native panel as inspection; this view owns no rail geometry. */
export class CommentsView {
	private readonly header = document.createElement('header');
	private readonly body = document.createElement('div');
	private readonly title = textElement('strong', 'Comments');
	private readonly status = textElement('p', '', 'ex-comment-status');
	private readonly composer = document.createElement('section');
	private readonly targetLabel = textElement('h3', '');
	private readonly quote = textElement('blockquote', '', 'ex-comment-quote');
	private readonly editor = document.createElement('textarea');
	private readonly submit = textElement('button', 'Add comment', 'ex-comment-save');
	private readonly answers = document.createElement('div');
	private readonly list = document.createElement('div');
	private readonly transfer = document.createElement('div');
	private readonly manual = document.createElement('textarea');
	private editing:
		| { readonly anchor: CommentAnchor; readonly commentId: string | undefined }
		| undefined;
	private closePanel: (() => void) | undefined;

	constructor(private readonly options: CommentsViewOptions) {
		this.header.className = 'ex-inspect-header';
		this.header.append(
			this.title,
			createIconButton('x', 'Close comments', (): void => this.closePanel?.()),
		);
		this.body.className = 'ex-inspect-body ex-comments-body';
		this.status.setAttribute('role', 'status');
		this.composer.className = 'ex-comment-composer';
		this.composer.hidden = true;
		this.editor.rows = 4;
		this.editor.placeholder = 'Leave a comment…';
		this.editor.setAttribute('aria-label', 'Your comment');
		this.submit.type = 'button';
		this.submit.disabled = true;
		this.submit.setAttribute('aria-keyshortcuts', 'Meta+Enter');
		this.submit.title = 'Save comment (⌘Enter)';
		const cancel = textElement('button', 'Cancel');
		cancel.type = 'button';
		cancel.addEventListener('click', (): void => this.endComposer());
		const actions = document.createElement('div');
		actions.className = 'ex-comment-actions';
		actions.append(cancel, this.submit);
		this.composer.append(this.targetLabel, this.quote, this.editor, actions);
		this.manual.readOnly = true;
		this.manual.hidden = true;
		this.manual.rows = 9;
		this.manual.setAttribute('aria-label', 'Review for manual copy');
		this.transfer.className = 'ex-comment-actions';
		const copy = textElement('button', 'Copy review');
		copy.type = 'button';
		copy.prepend(createPhosphorIcon('copy'));
		copy.addEventListener('click', (): void => {
			void this.copy();
		});
		const exportButton = textElement('button', 'Export review');
		exportButton.type = 'button';
		exportButton.prepend(createPhosphorIcon('download-simple'));
		exportButton.addEventListener('click', (): void => this.export());
		this.transfer.append(copy, exportButton);
		this.body.append(
			this.composer,
			this.answers,
			this.list,
			this.transfer,
			this.status,
			this.manual,
		);
		this.editor.addEventListener('input', (): void => {
			this.submit.disabled = !this.editor.value.trim();
		});
		this.editor.addEventListener('keydown', (event: KeyboardEvent): void => {
			if (
				event.key !== 'Enter' ||
				!event.metaKey ||
				event.shiftKey ||
				event.altKey ||
				event.ctrlKey ||
				event.isComposing
			)
				return;
			event.preventDefault();
			if (!this.submit.disabled) this.submit.click();
		});
		this.submit.addEventListener('click', (): void => {
			if (!this.editing || !this.editor.value.trim()) return;
			try {
				this.options.save(this.editing.anchor, this.editor.value, this.editing.commentId);
				this.endComposer();
				this.refresh();
			} catch (error) {
				this.showIssue(`Comment could not save: ${String(error)}`);
			}
		});
		this.refresh();
	}

	open(): void {
		if (this.closePanel) return;
		this.closePanel = openDrawer(
			'Comments',
			(): { header: HTMLElement; body: HTMLElement; restore: () => void } => ({
				header: this.header,
				body: this.body,
				restore: (): void => {
					this.closePanel = undefined;
					this.options.visibility(false);
				},
			}),
		);
		this.options.visibility(true);
	}
	toggle(): void {
		if (this.closePanel) this.closePanel();
		else this.open();
	}
	close(): void {
		this.closePanel?.();
	}

	compose(anchor: CommentAnchor, existing?: PageComment): void {
		this.open();
		this.editing = { anchor: structuredClone(anchor), commentId: existing?.commentId };
		this.composer.hidden = false;
		this.targetLabel.textContent = anchor.label;
		this.quote.hidden = anchor.kind !== 'text';
		this.quote.textContent = anchor.kind === 'text' ? anchor.selectedText : '';
		this.editor.value = existing?.body ?? '';
		this.submit.textContent = existing ? 'Save comment' : 'Add comment';
		this.submit.disabled = !this.editor.value.trim();
		this.options.composing(anchor);
		this.body.scrollTop = 0;
		this.editor.focus();
	}

	refresh(): void {
		const review = this.options.review();
		this.title.textContent = `Comments · ${review.comments.length}`;
		this.answers.replaceChildren();
		this.list.replaceChildren();
		if (review.answers.length) {
			this.answers.append(textElement('h3', 'Selected answers'));
			for (const answer of review.answers) {
				const item = document.createElement('article');
				item.append(
					textElement('p', answer.question, 'ex-review-question'),
					...answer.selected.map((option): HTMLElement => textElement('p', option.label)),
				);
				this.answers.append(item);
			}
		}
		if (!review.comments.length && !review.answers.length)
			this.list.append(textElement('p', 'No comments or selected answers yet.'));
		for (const comment of review.comments) {
			const item = document.createElement('article');
			item.className = 'ex-saved-comment';
			item.dataset['commentId'] = comment.commentId;
			const reveal = textElement('button', comment.anchor.label, 'ex-comment-anchor');
			reveal.type = 'button';
			reveal.setAttribute('aria-label', `Show anchor: ${comment.anchor.label}`);
			reveal.addEventListener('click', (): void => {
				const issue = this.options.reveal(comment);
				this.showIssue(issue);
				if (!issue && window.innerWidth < 900) this.closePanel?.();
			});
			item.append(reveal);
			if (comment.anchor.kind === 'text')
				item.append(textElement('blockquote', comment.anchor.selectedText, 'ex-comment-quote'));
			const edit = textElement('button', 'Edit', 'ex-comment-edit');
			edit.type = 'button';
			edit.setAttribute('aria-label', `Edit comment on ${comment.anchor.label}`);
			edit.addEventListener('click', (): void => this.compose(comment.anchor, comment));
			item.append(textElement('p', comment.body, 'ex-comment-body'), edit);
			this.list.append(item);
		}
		this.transfer.hidden = !review.comments.length && !review.answers.length;
		this.showIssue(this.options.issue());
	}

	private endComposer(): void {
		this.editing = undefined;
		this.composer.hidden = true;
		this.editor.value = '';
		this.options.composing(undefined);
	}
	private showIssue(message: string | undefined): void {
		this.status.textContent = message ?? '';
		this.status.hidden = !message;
	}
	private async copy(): Promise<void> {
		const text = formatReview(this.options.review());
		try {
			await navigator.clipboard.writeText(text);
			this.manual.hidden = true;
			this.showIssue('Review copied.');
		} catch {
			this.manual.value = text;
			this.manual.hidden = false;
			this.manual.focus();
			this.manual.select();
			this.showIssue('Clipboard is unavailable. Copy the review from the selected field.');
		}
	}
	private export(): void {
		const review = this.options.review();
		const url = URL.createObjectURL(
			new Blob([JSON.stringify(review, null, 2)], { type: 'application/json' }),
		);
		const link = document.createElement('a');
		link.href = url;
		link.download = `${review.documentId}-review.json`;
		document.body.append(link);
		link.click();
		link.remove();
		setTimeout((): void => URL.revokeObjectURL(url), 1000);
	}
}
