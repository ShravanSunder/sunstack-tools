import type { DrawerIsland } from '../../../../packages/demistify/src/islands/drawer.ts';
import { relatedRequirementMarkup, ruleRequirementMarkup } from './requirement-related-markup.js';
import { updateRequirementAddress, type RequirementStage } from './study-links.js';
import { icon, safeText } from './study-markup.js';
import type { RequirementExample } from './study-model.js';

interface RelatedViewContext {
	readonly need: RequirementExample;
	readonly stage: RequirementStage | undefined;
	readonly selectNeed: (id: string) => void;
}

type RelatedView = 'documents' | 'calls';

function readingAnchorForRecord(target: HTMLElement): HTMLElement {
	const disclosure = target.closest('details');
	const firstRecord = disclosure?.querySelector(':scope > .ex-trace-body > .lab-related-record');
	return target.closest('.lab-rule-reader') && firstRecord === target
		? (disclosure?.querySelector<HTMLElement>(':scope > summary') ?? target)
		: target;
}

/** A combined reading view, independent of the component's ordinary enlargement. */
export class RequirementRelatedView extends HTMLElement {
	private context: RelatedViewContext | undefined;
	private view: RelatedView = 'documents';
	private entryView: RelatedView = 'documents';
	private reading: 'documents' | 'rule-path' = 'documents';
	private returnNeed: string | undefined;
	private returnRecord: string | undefined;
	private initialized = false;

	configure(context: RelatedViewContext): void {
		this.context = context;
	}

	open(view: RelatedView = 'documents'): void {
		if (!this.context || this.querySelector('ex-drawer[open]')) return;
		this.view = view;
		this.entryView = view;
		this.reading =
			new URLSearchParams(location.search).get('reading') === 'rule-path'
				? 'rule-path'
				: 'documents';
		if (view === 'documents') this.writeReadingAddress();
		this.returnNeed = undefined;
		this.returnRecord = undefined;
		this.innerHTML =
			'<ex-drawer class="lab-related-drawer" label="Related documents" aria-labelledby="related-view-title"><header class="ex-inspect-header lab-related-header"></header><div class="ex-inspect-body lab-related-content"></div></ex-drawer>';
		const drawer = this.querySelector<DrawerIsland>('ex-drawer');
		drawer?.addEventListener('close', (): void => {
			const context = this.context;
			if (!context || !this.isConnected) return;
			const address = new URL(location.href);
			address.searchParams.delete('reading');
			history.replaceState(history.state, '', address);
			updateRequirementAddress(context.need.id, false, context.stage);
			const returnNeed = this.returnNeed;
			this.replaceChildren();
			if (returnNeed) context.selectNeed(returnNeed);
		});
		this.renderContent();
		drawer?.show();
	}

	disconnectedCallback(): void {
		this.querySelector<DrawerIsland>('ex-drawer')?.close();
	}

	connectedCallback(): void {
		if (this.initialized) return;
		this.initialized = true;
		this.addEventListener('click', (event): void => {
			if (!(event.target instanceof Element) || !this.context) return;
			// The parent owns the normal pane; actions in this workspace stay local.
			event.stopPropagation();
			const action = event.target.closest<HTMLElement>('[data-action]')?.dataset['action'];
			const reading = event.target.closest<HTMLElement>('[data-reading]')?.dataset['reading'];
			if (reading === 'documents' || reading === 'rule-path') {
				this.reading = reading;
				this.writeReadingAddress();
				this.renderContent();
				this.querySelector<HTMLElement>(`[data-reading="${reading}"]`)?.focus({
					preventScroll: true,
				});
				return;
			}
			if (action === 'close-related') {
				this.querySelector<DrawerIsland>('ex-drawer')?.close();
				return;
			}
			if (action === 'open-calls' || action === 'back-from-calls') {
				if (action === 'back-from-calls' && this.entryView === 'calls') {
					this.querySelector<DrawerIsland>('ex-drawer')?.close();
					return;
				}
				if (action === 'open-calls')
					this.returnRecord =
						event.target.closest<HTMLElement>('[data-record-id]')?.dataset['recordId'];
				this.view = action === 'open-calls' ? 'calls' : 'documents';
				updateRequirementAddress(this.context.need.id, false, this.context.stage);
				if (this.view === 'documents') this.writeReadingAddress();
				this.renderContent();
				const content = this.querySelector<HTMLElement>('.lab-related-content');
				if (content) content.scrollTop = 0;
				if (action === 'back-from-calls' && this.returnRecord) {
					const target = this.querySelector<HTMLElement>(
						`#related-record-${CSS.escape(this.returnRecord)}`,
					);
					const disclosure = target?.closest('details');
					if (disclosure) disclosure.open = true;
					if (target) readingAnchorForRecord(target).scrollIntoView({ block: 'start' });
					target
						?.querySelector<HTMLElement>('[data-action="open-calls"]')
						?.focus({ preventScroll: true });
					this.returnRecord = undefined;
					return;
				}
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
				if (target) readingAnchorForRecord(target).scrollIntoView({ block: 'start' });
				target?.focus({ preventScroll: true });
				return;
			}
			const callLink = event.target.closest<HTMLAnchorElement>(
				'.lab-frame-requirement a, lab-execution-review a.lab-navigation-action',
			);
			const needId = callLink
				? new URL(callLink.href).searchParams.get('need')
				: event.target.closest<HTMLElement>('[data-need]')?.dataset['need'];
			if (!needId) return;
			event.preventDefault();
			this.returnNeed = needId;
			this.querySelector<DrawerIsland>('ex-drawer')?.close();
		});
	}

	private writeReadingAddress(): void {
		const address = new URL(location.href);
		address.searchParams.set('reading', this.reading);
		history.replaceState(history.state, '', address);
	}

	private renderContent(): void {
		if (!this.context) return;
		const need = this.context.need;
		const calls = this.view === 'calls';
		this.querySelector<DrawerIsland>('ex-drawer')?.classList.toggle('lab-call-workspace', calls);
		const header = this.querySelector('.lab-related-header');
		if (header) {
			const returnLabel =
				this.entryView === 'calls' ? 'Back to Program Design' : 'Back to related documents';
			header.innerHTML = `${calls ? `<button type="button" class="lab-navigation-action lab-icon-action" data-action="back-from-calls" aria-label="${returnLabel}" title="${returnLabel}">${icon('arrow-left')}</button>` : ''}<div class="lab-related-title"><small>${need.id} · ${calls ? 'Call stack' : 'Related documents'}</small><div class="lab-local-heading" id="related-view-title" role="heading" aria-level="2">${safeText(need.title)}</div></div><button type="button" class="lab-navigation-action lab-icon-action" data-action="close-related" aria-label="${calls ? 'Close call stack' : 'Close related documents'}" title="Close" autofocus>${icon('x')}</button>`;
		}
		const content = this.querySelector('.lab-related-content');
		if (!content) return;
		content.innerHTML = calls
			? '<lab-execution-review></lab-execution-review>'
			: `<div class="lab-reading-experiment" data-width="${this.reading === 'rule-path' ? 'reading' : 'canvas'}"><span>Reading experiment</span><div class="lab-reading-options lab-toggle-group" role="group" aria-label="Reading experiment"><button type="button" data-reading="documents" aria-pressed="${this.reading === 'documents'}">1 · Documents</button><button type="button" data-reading="rule-path" aria-pressed="${this.reading === 'rule-path'}">2 · Follow the rule</button></div></div>${this.reading === 'rule-path' ? ruleRequirementMarkup(this.context.need) : relatedRequirementMarkup(this.context.need)}`;
	}
}
