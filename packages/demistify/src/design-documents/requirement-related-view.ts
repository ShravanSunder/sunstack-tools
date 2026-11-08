import type { DrawerElement } from '../elements.ts';
import type { RequirementExample } from './collection-model.js';
import { updateRequirementAddress, type RequirementStage } from './document-links.js';
import { icon, safeText } from './document-markup.js';
import { relatedRequirementMarkup } from './requirement-related-markup.js';

interface RelatedViewContext {
	readonly need: RequirementExample;
	readonly stage: RequirementStage | undefined;
	readonly selectNeed: (id: string) => void;
}

type RelatedView = 'documents' | 'calls';

/** A combined reading view, independent of the component's ordinary enlargement. */
export class RequirementRelatedView extends HTMLElement {
	private context: RelatedViewContext | undefined;
	private view: RelatedView = 'documents';
	private entryView: RelatedView = 'documents';
	private returnNeed: string | undefined;
	private initialized = false;

	configure(context: RelatedViewContext): void {
		this.context = context;
	}

	open(view: RelatedView = 'documents'): void {
		if (!this.context || this.querySelector('ex-drawer[open]')) return;
		this.view = view;
		this.entryView = view;
		if (view === 'documents') this.writeReadingAddress();
		this.returnNeed = undefined;
		this.innerHTML =
			'<ex-drawer class="ex-design-related-drawer" label="Related documents" aria-labelledby="related-view-title"><header class="ex-inspect-header ex-design-related-header"></header><div class="ex-inspect-body ex-design-related-content"></div></ex-drawer>';
		const drawer = this.querySelector<DrawerElement>('ex-drawer');
		drawer?.addEventListener('close', (): void => {
			const context = this.context;
			if (!context) return;
			const address = new URL(location.href);
			address.searchParams.delete('reading');
			history.replaceState(history.state, '', address);
			if (!this.isConnected) return;
			updateRequirementAddress(context.need.id, false, context.stage);
			const returnNeed = this.returnNeed;
			this.replaceChildren();
			if (returnNeed) context.selectNeed(returnNeed);
		});
		this.renderContent();
		drawer?.show();
	}

	disconnectedCallback(): void {
		this.querySelector<DrawerElement>('ex-drawer')?.close();
	}

	connectedCallback(): void {
		if (this.initialized) return;
		this.initialized = true;
		this.addEventListener('click', (event): void => {
			if (!(event.target instanceof Element) || !this.context) return;
			// The parent owns the normal pane; actions in this workspace stay local.
			event.stopPropagation();
			const action = event.target.closest<HTMLElement>('[data-action]')?.dataset['action'];
			if (action === 'close-related') {
				this.querySelector<DrawerElement>('ex-drawer')?.close();
				return;
			}
			if (action === 'open-calls' || action === 'back-from-calls') {
				if (action === 'back-from-calls' && this.entryView === 'calls') {
					this.querySelector<DrawerElement>('ex-drawer')?.close();
					return;
				}
				this.view = action === 'open-calls' ? 'calls' : 'documents';
				updateRequirementAddress(this.context.need.id, false, this.context.stage);
				if (this.view === 'documents') this.writeReadingAddress();
				this.renderContent();
				const content = this.querySelector<HTMLElement>('.ex-design-related-content');
				if (content) content.scrollTop = 0;
				this.querySelector<HTMLElement>(
					'[data-action="back-from-calls"], [data-action="open-calls"]',
				)?.focus({ preventScroll: true });
				return;
			}
			const source = event.target.closest<HTMLAnchorElement>('a[href^="#related-record-"]');
			if (source) {
				event.preventDefault();
				const target = this.querySelector<HTMLElement>(`#${CSS.escape(source.hash.slice(1))}`);
				const disclosure = target?.closest('details');
				if (disclosure) disclosure.open = true;
				if (target) target.scrollIntoView({ block: 'start' });
				target?.focus({ preventScroll: true });
				return;
			}
			const callLink = event.target.closest<HTMLAnchorElement>(
				'.ex-design-frame-requirement a, ex-call-stack a.ex-design-navigation-action',
			);
			const needId = callLink
				? new URL(callLink.href).searchParams.get('need')
				: event.target.closest<HTMLElement>('[data-need]')?.dataset['need'];
			if (!needId) return;
			event.preventDefault();
			this.returnNeed = needId;
			this.querySelector<DrawerElement>('ex-drawer')?.close();
		});
	}

	private writeReadingAddress(): void {
		const address = new URL(location.href);
		address.searchParams.set('reading', 'documents');
		history.replaceState(history.state, '', address);
	}

	private renderContent(): void {
		if (!this.context) return;
		const need = this.context.need;
		const calls = this.view === 'calls';
		this.querySelector<DrawerElement>('ex-drawer')?.classList.toggle(
			'ex-design-call-workspace',
			calls,
		);
		const header = this.querySelector('.ex-design-related-header');
		if (header) {
			const returnLabel =
				this.entryView === 'calls' ? 'Back to Program Design' : 'Back to related documents';
			header.innerHTML = `${calls ? `<button type="button" class="ex-design-navigation-action ex-design-icon-action" data-action="back-from-calls" aria-label="${returnLabel}" title="${returnLabel}">${icon('arrow-left')}</button>` : ''}<div class="ex-design-related-title"><small>${need.id} · ${calls ? 'Call stack' : 'Related documents'}</small><div class="ex-design-local-heading" id="related-view-title" role="heading" aria-level="2">${safeText(need.title)}</div></div><button type="button" class="ex-design-navigation-action ex-design-icon-action" data-action="close-related" aria-label="${calls ? 'Close call stack' : 'Close related documents'}" title="Close" autofocus>${icon('x')}</button>`;
		}
		const content = this.querySelector('.ex-design-related-content');
		if (!content) return;
		content.innerHTML = calls
			? '<ex-call-stack></ex-call-stack>'
			: relatedRequirementMarkup(this.context.need);
	}
}
