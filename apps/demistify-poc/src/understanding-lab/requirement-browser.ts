import { readCollection } from './collection-data.js';
import type { RequirementRelatedView } from './requirement-related-view.js';
import { stageMarkup, scenarioBadges } from './requirement-stages.js';
import type { SearchPicker } from './search-picker.js';
import {
	revealStudyFragment,
	updateRequirementAddress,
	updateRequirementListAddress,
	type RequirementStage,
} from './study-links.js';
import { icon, restoreStudyFocusAfterRender, safeText } from './study-markup.js';
import { loadStudyModel, type RequirementExample } from './study-model.js';

const { requirements } = loadStudyModel();
const documentViews = [
	{ id: 'requirement', label: 'Overview' },
	{ id: 'specification', label: 'Specification' },
	{ id: 'program-design', label: 'Program Design' },
	{ id: 'verification', label: 'Verification' },
] as const;

/** The normal browser owns the selected need and document; enlargement keeps this same view. */
export class RequirementBrowser extends HTMLElement {
	private selectedId = 'U1';
	private capability = 'All needs';
	private stage: RequirementStage | undefined;
	private mobileView: 'list' | 'detail' = 'list';
	private historyEvents: AbortController | undefined;
	private initialized = false;

	connectedCallback(): void {
		if (this.initialized) {
			this.bindHistory();
			return;
		}
		this.initialized = true;
		this.tabIndex = 0;
		this.setAttribute('role', 'group');
		this.setAttribute('aria-label', 'Requirements browser');
		this.setAttribute('aria-keyshortcuts', 'ArrowLeft ArrowRight');
		this.classList.add('lab-focus-surface');
		const requested = new URLSearchParams(location.search).get('need');
		if (requested && requirements.some((item): boolean => item.id === requested)) {
			this.selectedId = requested;
			this.mobileView = 'detail';
		}
		this.readStage();
		this.addEventListener('picker-select', (event): void => {
			if (!(event instanceof CustomEvent)) return;
			const group: unknown = event.detail?.value;
			if (typeof group !== 'string' || !this.validCapability(group)) return;
			this.capability = group;
			const visible = this.visibleNeeds();
			if (!visible.some((item): boolean => item.id === this.selectedId) && visible[0])
				this.selectedId = visible[0].id;
			if (this.compact() && this.mobileView === 'list') updateRequirementListAddress();
			else updateRequirementAddress(this.selectedId, false, this.stage);
			this.render();
			this.querySelector<HTMLElement>('lab-search-picker summary')?.focus({ preventScroll: true });
		});
		this.addEventListener('click', (event): void => {
			if (!(event.target instanceof Element)) return;
			const action = event.target.closest<HTMLElement>('[data-action]')?.dataset['action'];
			if (action === 'view-related') {
				this.querySelector<RequirementRelatedView>('lab-requirement-related')?.open();
				return;
			}
			if (action === 'open-calls') {
				this.querySelector<RequirementRelatedView>('lab-requirement-related')?.open('calls');
				return;
			}
			const documentControl = event.target.closest<HTMLElement>('[data-document]');
			const tab = documentViews.find((item) => item.id === documentControl?.dataset['document']);
			if (tab) {
				const recordTarget = documentControl?.dataset['recordTarget'];
				this.stage = tab.id === 'requirement' ? undefined : tab.id;
				this.chooseNeed(documentControl?.dataset['contextNeed'] ?? this.selectedId);
				if (recordTarget)
					requestAnimationFrame((): void => {
						const heading = this.querySelector<HTMLElement>(`#record-${CSS.escape(recordTarget)}`);
						if (!heading) return;
						heading.tabIndex = -1;
						heading.focus({ preventScroll: true });
						heading.scrollIntoView({ block: 'nearest' });
						const address = new URL(location.href);
						address.hash = `record-${recordTarget}`;
						history.replaceState(history.state, '', address);
					});
				else if (documentControl?.closest('.lab-document-switcher'))
					requestAnimationFrame((): void => {
						const selected = this.querySelector<HTMLElement>(
							'.lab-document-switcher button[aria-pressed="true"]',
						);
						selected?.focus({ preventScroll: true });
						selected?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
					});
				return;
			}
			if (action === 'back-to-needs') {
				this.mobileView = 'list';
				updateRequirementListAddress();
				this.render();
				this.focusList();
				return;
			}
			if (action === 'previous-need' || action === 'next-need') {
				this.moveNeed(action === 'previous-need' ? -1 : 1);
				return;
			}
			const id = event.target.closest<HTMLElement>('[data-need]')?.dataset['need'];
			if (id) this.chooseNeed(id);
		});
		this.addEventListener('keydown', (event): void => {
			if (
				event.defaultPrevented ||
				event.altKey ||
				event.ctrlKey ||
				event.metaKey ||
				!(event.target instanceof Element)
			)
				return;
			if (event.target.closest('.lab-document-switcher button[data-document]')) {
				const tabs = [
					...this.querySelectorAll<HTMLButtonElement>(
						'.lab-document-switcher button[data-document]',
					),
				];
				const index = tabs.findIndex((tab) => tab === event.target);
				const next =
					event.key === 'Home'
						? tabs[0]
						: event.key === 'End'
							? tabs.at(-1)
							: event.key === 'ArrowLeft'
								? tabs[(index - 1 + tabs.length) % tabs.length]
								: event.key === 'ArrowRight'
									? tabs[(index + 1) % tabs.length]
									: undefined;
				if (next) {
					event.preventDefault();
					next.click();
				}
				return;
			}
			if (
				event.target.closest(
					'lab-execution-review,lab-search-picker,input,textarea,select,[contenteditable],dialog,ex-drawer,ex-comments,summary',
				) ||
				document.getSelection()?.isCollapsed === false
			)
				return;
			if (this.compact() && this.mobileView === 'list') return;
			if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
			event.preventDefault();
			this.moveNeed(
				event.key === 'ArrowLeft' ? -1 : 1,
				Boolean(event.target.closest('[data-need]')),
			);
		});
		this.bindHistory();
		this.render();
		const reading = new URLSearchParams(location.search).get('reading');
		if (reading === 'documents' || reading === 'rule-path') {
			this.querySelector<RequirementRelatedView>('lab-requirement-related')?.open();
			return;
		}
		if (this.mobileView === 'detail' && this.compact() && !location.hash) this.focusDetail();
		else revealStudyFragment(this);
	}

	disconnectedCallback(): void {
		this.historyEvents?.abort();
	}

	private readStage(): void {
		const address = new URL(location.href);
		const page = address.pathname.split('/').at(-1)?.replace('.html', '');
		const requested =
			address.searchParams.get('stage') ?? (page !== 'requirements' ? page : undefined);
		this.stage =
			requested === 'specification' ||
			requested === 'program-design' ||
			requested === 'verification'
				? requested
				: undefined;
	}

	private validCapability(value: string): boolean {
		return value === 'All needs' || requirements.some((item) => item.capability === value);
	}
	private visibleNeeds(): readonly RequirementExample[] {
		return requirements.filter(
			(item) => this.capability === 'All needs' || item.capability === this.capability,
		);
	}
	private compact(): boolean {
		const layout = this.querySelector('.lab-requirement-layout');
		return Boolean(
			layout && getComputedStyle(layout).getPropertyValue('--requirements-compact').trim() === '1',
		);
	}
	private chooseNeed(id: string): void {
		if (!requirements.some((item) => item.id === id)) return;
		if (!this.visibleNeeds().some((item) => item.id === id)) this.capability = 'All needs';
		this.selectedId = id;
		this.mobileView = 'detail';
		updateRequirementAddress(id, true, this.stage);
		this.render();
		if (this.compact()) this.focusDetail();
	}
	private bindHistory(): void {
		this.historyEvents?.abort();
		this.historyEvents = new AbortController();
		window.addEventListener(
			'popstate',
			(event): void => {
				const id = new URLSearchParams(location.search).get('need');
				const valid = id && requirements.some((item) => item.id === id);
				if (valid) this.selectedId = id;
				this.readStage();
				this.mobileView = valid ? 'detail' : 'list';
				const group: unknown = event.state?.labNeedCapability;
				if (typeof group === 'string' && this.validCapability(group)) this.capability = group;
				if (!this.visibleNeeds().some((item) => item.id === this.selectedId))
					this.capability = 'All needs';
				this.render();
			},
			{ signal: this.historyEvents.signal },
		);
	}
	private focusDetail(): void {
		requestAnimationFrame((): void => {
			this.querySelector<HTMLElement>('.lab-document-panel')?.focus({ preventScroll: true });
			this.querySelector('.lab-requirement-shell')?.scrollIntoView({ block: 'start' });
		});
	}
	private focusList(): void {
		requestAnimationFrame((): void => {
			const selected = this.querySelector<HTMLButtonElement>(`[data-need="${this.selectedId}"]`);
			selected?.focus({ preventScroll: true });
			selected?.scrollIntoView({ block: 'nearest' });
		});
	}
	private moveNeed(direction: number, focusSelected = false): void {
		const visible = this.visibleNeeds();
		const index = visible.findIndex((item) => item.id === this.selectedId);
		const next = visible[index + direction];
		if (!next) return;
		this.chooseNeed(next.id);
		if (!this.compact() && focusSelected) this.focusList();
	}

	private documentPane(need: RequirementExample): string {
		const active = this.stage ?? 'requirement';
		return `<div class="lab-document-header"><header class="lab-document-subject"><small>${need.id} · ${safeText(need.capability)}</small><div class="lab-local-heading" role="heading" aria-level="3">${safeText(need.title)}</div></header><div class="lab-document-toolbar"><div class="lab-document-switcher lab-toggle-group" role="group" aria-label="Document view">${documentViews.map((tab) => `<button type="button" data-document="${tab.id}" id="document-view-${tab.id}" aria-pressed="${active === tab.id}" aria-controls="requirement-document-panel" tabindex="${active === tab.id ? '0' : '-1'}">${tab.label}</button>`).join('')}</div><div class="lab-document-actions">${this.stage === 'program-design' ? `<button type="button" class="lab-navigation-action lab-icon-action" data-action="open-calls" aria-label="Open call stack" title="Call stack">${icon('git-branch')}</button>` : ''}<button type="button" class="lab-navigation-action lab-icon-action lab-view-together" data-action="view-related" aria-label="View together" title="View together">${icon('squares-four')}</button></div></div></div><section class="lab-document-panel" id="requirement-document-panel" role="region" aria-labelledby="document-view-${active}" tabindex="-1">${this.stage ? stageMarkup(need, this.stage) : `<p class="lab-need-statement">“${safeText(need.need)}”</p>${stageMarkup(need, undefined)}`}</section>`;
	}

	private render(): void {
		const restoreFocus = restoreStudyFocusAfterRender(this);
		this.dataset['mobileView'] = this.mobileView;
		this.dataset['stagePage'] = String(Boolean(this.stage));
		const visible = this.visibleNeeds();
		const selected = requirements.find((item) => item.id === this.selectedId);
		if (!selected) return;
		const position = visible.findIndex((item) => item.id === selected.id);
		const capabilities = ['All needs', ...new Set(requirements.map((item) => item.capability))];
		const groups = [...new Set(visible.map((item) => item.capability))];
		this.innerHTML = `<div class="lab-requirement-shell"><div class="lab-component-toolbar lab-requirement-toolbar"><div class="lab-requirement-list-controls"><lab-search-picker></lab-search-picker></div><button type="button" class="lab-back-to-needs" data-action="back-to-needs">${icon('arrow-left')} Back to needs</button></div><div class="lab-requirement-layout"><nav class="lab-requirement-list" aria-label="User needs">${groups
			.map(
				(group) =>
					`<section><div class="lab-local-heading" role="heading" aria-level="3">${safeText(group)}</div>${visible
						.filter((item) => item.capability === group)
						.map(
							(item) =>
								`<button type="button" data-need="${item.id}" aria-current="${item.id === selected.id}"><span class="lab-need-title"><span class="lab-need-id">${item.id}</span><strong>${safeText(item.title)}</strong></span><span class="lab-need-description"><span>${safeText(item.need)}</span>${scenarioBadges(readCollection(), item.id)}</span></button>`,
						)
						.join('')}</section>`,
			)
			.join(
				'',
			)}</nav><section class="lab-requirement-detail" id="need-${selected.id}" aria-live="polite">${this.documentPane(selected)}</section></div><div class="lab-component-nav lab-requirement-pagination" role="group" aria-label="Requirement navigation"><details class="lab-help"><summary>${icon('question')} Help</summary><p>Left and Right choose needs. In the document view controls, use Left/Right or Home/End to choose a view. View together opens the related-document comparison. In the call browser, select a call to keep its details open.</p></details><button type="button" data-action="previous-need" ${position <= 0 ? 'disabled' : ''}>${icon('arrow-left')} Previous</button><span class="lab-position" role="status">${position + 1} of ${visible.length}</span><button type="button" data-action="next-need" ${position >= visible.length - 1 ? 'disabled' : ''}>Next ${icon('arrow-right')}</button></div></div><lab-requirement-related></lab-requirement-related>`;
		this.querySelector<SearchPicker>('lab-search-picker')?.configure({
			label: 'Choose a capability',
			placeholder: 'Search capabilities…',
			icon: 'squares-four',
			selected: this.capability,
			options: capabilities.map((value) => {
				const count =
					value === 'All needs'
						? requirements.length
						: requirements.filter((item) => item.capability === value).length;
				return {
					value,
					label: value === 'All needs' ? 'All capabilities' : value,
					detail: `${count} ${count === 1 ? 'need' : 'needs'}`,
				};
			}),
		});
		this.querySelector<RequirementRelatedView>('lab-requirement-related')?.configure({
			need: selected,
			stage: this.stage,
			selectNeed: (id): void => {
				this.stage = undefined;
				this.chooseNeed(id);
			},
		});
		restoreFocus();
	}
}
