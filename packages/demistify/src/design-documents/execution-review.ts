import type { DrawerElement } from '../elements.ts';
import { loadDocumentModel, type CallStackExample, type FrameExample } from './collection-model.js';
import { requirementHref, updateCallAddress } from './document-links.js';
import { icon, restoreDocumentFocusAfterRender, safeText } from './document-markup.js';
import { callStackMarkup, frameColor, frameDetailMarkup } from './execution-markup.js';
import type { SearchPicker } from './search-picker.js';

type CallSelection =
	| { readonly kind: 'frame'; readonly frameId: string }
	| { readonly kind: 'unmapped-need'; readonly needId: string }
	| { readonly kind: 'missing-call'; readonly requestedId: string };

/** One stack selection owns the nested path; hover previews detail without changing its meaning. */
export class ExecutionReview extends HTMLElement {
	private readonly model = loadDocumentModel();
	private stackId =
		this.model.entry.calls.kind === 'available' ? this.model.entry.calls.stackId : '';
	private selection: CallSelection =
		this.model.entry.calls.kind === 'available'
			? { kind: 'frame', frameId: this.model.entry.calls.frameId }
			: { kind: 'missing-call', requestedId: '' };
	private hoveredFrameId: string | undefined;
	private pendingHoverId: string | undefined;
	private hoverTimer: ReturnType<typeof setTimeout> | undefined;
	private expandedSupport = false;
	private initialized = false;

	private get stack(): CallStackExample {
		const stack = this.model.callStacks.find((item): boolean => item.id === this.stackId);
		if (!stack) throw new Error('Unknown example call stack.');
		return stack;
	}

	connectedCallback(): void {
		if (this.initialized) return;
		this.initialized = true;
		this.setAttribute('role', 'group');
		this.setAttribute('aria-label', this.getAttribute('aria-label') ?? 'Call stack browser');
		if (this.model.entry.calls.kind === 'empty') {
			this.innerHTML = '<p role="status">No call paths are supplied.</p>';
			return;
		}
		this.tabIndex = 0;
		this.setAttribute('aria-keyshortcuts', 'ArrowLeft ArrowRight ArrowUp ArrowDown');
		this.classList.add('ex-design-focus-surface');
		this.readAddress();
		this.addEventListener('picker-select', (event): void => {
			if (!(event instanceof CustomEvent) || typeof event.detail?.value !== 'string') return;
			this.chooseStack(event.detail.value, true);
		});
		this.addEventListener('click', (event): void => {
			if (
				event.target instanceof Element &&
				event.target.closest('[data-action="close-call-details"]')
			) {
				this.querySelector<DrawerElement>('.ex-design-call-detail-sheet')?.close();
				return;
			}
			const frame = this.frameFrom(event.target);
			if (frame) {
				this.chooseFrame(frame.id);
				this.openCompactDetails();
			}
		});
		this.addEventListener('pointermove', (event): void => {
			if (event.pointerType === 'touch') return;
			const frame = this.frameFrom(event.target);
			if (!frame) {
				if (!(event.target instanceof Element) || !event.target.closest('.ex-design-frame-detail'))
					this.clearPreview();
				return;
			}
			if (this.selection.kind === 'frame' && frame.id === this.selection.frameId) {
				this.clearPreview();
				return;
			}
			if (frame.id === this.hoveredFrameId || frame.id === this.pendingHoverId) return;
			clearTimeout(this.hoverTimer);
			this.pendingHoverId = frame.id;
			this.hoverTimer = setTimeout((): void => {
				this.hoveredFrameId = frame.id;
				this.pendingHoverId = undefined;
				this.updatePanels();
			}, 100);
		});
		this.addEventListener('pointerleave', (): void => this.clearPreview());
		this.addEventListener(
			'toggle',
			(event): void => {
				if (
					!(event.target instanceof HTMLDetailsElement) ||
					!event.target.closest('ex-design-hidden-calls')
				)
					return;
				this.expandedSupport = event.target.open;
				if (
					!event.target.open &&
					this.stack.frames.find((frame): boolean => frame.id === this.hoveredFrameId)?.supporting
				)
					this.clearPreview();
				this.updatePanels();
			},
			true,
		);
		this.addEventListener('keydown', (event): void => this.onKey(event));
		this.render();
		const selectedIndex = this.selectedIndex();
		if (new URLSearchParams(location.search).has('call') && selectedIndex >= 0)
			requestAnimationFrame((): void =>
				this.querySelector(`[data-frame="${selectedIndex}"]`)?.scrollIntoView({ block: 'nearest' }),
			);
	}

	disconnectedCallback(): void {
		clearTimeout(this.hoverTimer);
		this.querySelector<DrawerElement>('.ex-design-call-detail-sheet[open]')?.close();
	}

	private openCompactDetails(): void {
		const layout = this.querySelector('.ex-design-execution-layout');
		if (
			!layout ||
			getComputedStyle(layout).getPropertyValue('--call-details-overlay').trim() !== '1'
		)
			return;
		const sheet = this.querySelector<DrawerElement>('.ex-design-call-detail-sheet');
		const detail = this.querySelector('.ex-design-frame-detail');
		const body = sheet?.querySelector('.ex-design-call-detail-body');
		if (!sheet || sheet.open || !detail || !body) return;
		body.append(detail);
		sheet.show();
	}

	private readAddress(): void {
		const params = new URLSearchParams(location.search);
		const requestedStack = this.model.callStacks.find(
			(item): boolean => item.id === params.get('stack'),
		);
		if (requestedStack) {
			this.stackId = requestedStack.id;
			const last = requestedStack.frames.at(-1);
			if (last) this.selection = { kind: 'frame', frameId: last.id };
		}
		const needId = params.get('need');
		if (
			needId &&
			this.model.requirements.some((item): boolean => item.id === needId) &&
			!params.has('call')
		) {
			const owner =
				(requestedStack?.frames.some((frame): boolean => frame.requirement === needId)
					? requestedStack
					: undefined) ??
				this.model.callStacks.find((item): boolean => item.primaryNeed === needId) ??
				this.model.callStacks.find((item): boolean =>
					item.frames.some((frame): boolean => frame.requirement === needId),
				);
			const frame = owner?.frames.toReversed().find((item): boolean => item.requirement === needId);
			if (owner && frame) {
				this.stackId = owner.id;
				this.selection = { kind: 'frame', frameId: frame.id };
			} else this.selection = { kind: 'unmapped-need', needId };
		}
		const callId = params.get('call');
		if (callId)
			this.selection = this.stack.frames.some((frame): boolean => frame.id === callId)
				? { kind: 'frame', frameId: callId }
				: { kind: 'missing-call', requestedId: callId };
		this.expandedSupport = Boolean(this.stack.frames[this.selectedIndex()]?.supporting);
	}

	private selectedIndex(): number {
		const selection = this.selection;
		return selection.kind === 'frame'
			? this.stack.frames.findIndex((frame): boolean => frame.id === selection.frameId)
			: -1;
	}
	private frameFrom(target: EventTarget | null): FrameExample | undefined {
		if (!(target instanceof Element)) return undefined;
		if (target.closest('[data-comment-slot],[data-comment-control],.ex-entity-comment'))
			return undefined;
		const card = target.closest<HTMLElement>('[data-call-index]');
		return card && this.contains(card)
			? this.stack.frames[Number(card.dataset['callIndex'])]
			: undefined;
	}
	private clearPreview(): void {
		clearTimeout(this.hoverTimer);
		this.pendingHoverId = undefined;
		if (this.hoveredFrameId !== undefined) {
			this.hoveredFrameId = undefined;
			this.updatePanels();
		}
	}
	private chooseFrame(frameId: string): void {
		if (!this.stack.frames.some((frame): boolean => frame.id === frameId)) return;
		this.clearPreview();
		this.selection = { kind: 'frame', frameId };
		this.writeAddress(frameId);
		this.updatePanels();
	}
	private chooseStack(stackId: string, focusPicker = false): void {
		const stack = this.model.callStacks.find((item): boolean => item.id === stackId);
		const last = stack?.frames.at(-1);
		if (!stack || !last) return;
		this.clearPreview();
		this.stackId = stack.id;
		this.selection = { kind: 'frame', frameId: last.id };
		this.expandedSupport = false;
		this.writeAddress(last.id);
		this.render();
		if (focusPicker)
			this.querySelector<HTMLElement>('ex-design-picker summary')?.focus({ preventScroll: true });
		else this.focus({ preventScroll: true });
	}
	private writeAddress(frameId: string): void {
		// Embedded calls keep local selection; the document browser owns its address.
		if (this.closest('ex-design-related')) return;
		updateCallAddress(this.stack.id, frameId);
	}
	private onKey(event: KeyboardEvent): void {
		if (
			event.defaultPrevented ||
			event.altKey ||
			event.ctrlKey ||
			event.metaKey ||
			!(event.target instanceof Element)
		)
			return;
		if (
			event.target.closest(
				'ex-design-picker,input,textarea,select,[contenteditable],ex-comments,summary,[data-comment-slot],[data-comment-control]',
			) ||
			this.querySelector('.ex-design-call-detail-sheet[open]') ||
			document.getSelection()?.isCollapsed === false
		)
			return;
		if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key)) return;
		event.preventDefault();
		if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
			const index = this.model.callStacks.findIndex((item): boolean => item.id === this.stackId);
			const next = this.model.callStacks[index + (event.key === 'ArrowLeft' ? -1 : 1)];
			if (next) this.chooseStack(next.id);
		} else {
			const visible = this.stack.frames.filter(
				(frame): boolean => this.expandedSupport || !frame.supporting,
			);
			const current = this.selection;
			const index =
				current.kind === 'frame'
					? visible.findIndex((frame): boolean => frame.id === current.frameId)
					: -1;
			const next = visible[index < 0 ? 0 : index + (event.key === 'ArrowUp' ? -1 : 1)];
			if (!next) return;
			this.chooseFrame(next.id);
			const button = this.querySelector<HTMLButtonElement>(
				`[data-frame="${this.selectedIndex()}"]`,
			);
			button?.focus({ preventScroll: true });
			button?.scrollIntoView({ block: 'nearest' });
		}
	}

	private updatePanels(): void {
		const selected = this.stack.frames[this.selectedIndex()];
		const preview = this.stack.frames.find((frame): boolean => frame.id === this.hoveredFrameId);
		const frame = preview ?? selected;
		const detail = this.querySelector<HTMLElement>('.ex-design-frame-detail');
		if (!detail) return;
		const identity = `${this.stackId}:${frame?.id ?? ''}:${preview !== undefined}:${JSON.stringify(this.selection)}`;
		if (detail.dataset['context'] !== identity) {
			detail.dataset['context'] = identity;
			detail.id = `call-${this.stackId}-${frame?.id ?? (this.selection.kind === 'unmapped-need' ? this.selection.needId : 'missing')}`;
			detail.style.setProperty('--frame-color', frame ? frameColor(frame.category) : 'var(--rule)');
			if (frame) detail.innerHTML = frameDetailMarkup(frame, preview !== undefined);
			else if (this.selection.kind === 'unmapped-need') {
				const needId = this.selection.needId;
				const need = this.model.requirements.find((item): boolean => item.id === needId);
				detail.innerHTML = `<small>${safeText(needId)} · Call mapping</small><div class="ex-design-local-heading" role="heading" aria-level="3">No call is mapped to this need</div><p>${safeText(need?.owner ?? '')}</p><a class="ex-design-navigation-action" href="${requirementHref(needId)}">${icon('arrow-left')} Open the original requirement</a>`;
			} else
				detail.innerHTML =
					'<div class="ex-design-local-heading" role="heading" aria-level="3">Call not found in this stack</div><p>Choose an existing call from the nested list.</p>';
		}
		const hidden = this.querySelector('.ex-design-hidden-selection');
		if (hidden)
			hidden.textContent =
				!this.expandedSupport && selected?.supporting ? `Selected: ${selected.name}` : '';
		for (const button of this.querySelectorAll<HTMLButtonElement>('button[data-frame]'))
			button.setAttribute(
				'aria-pressed',
				String(Number(button.dataset['frame']) === this.selectedIndex()),
			);
	}

	private render(): void {
		const restoreFocus = restoreDocumentFocusAfterRender(this);
		this.innerHTML = `<div class="ex-design-component-toolbar"><ex-design-picker></ex-design-picker><details class="ex-design-help"><summary>${icon('question')} Help</summary><p>Hover to preview on wide screens. Select a call for details; Up/Down select a row and Enter opens it on narrow screens. Left/Right choose a stack while focused here. Comments are separate controls.</p></details></div><p class="ex-design-soft">${safeText(this.stack.description)} Select a call for details.</p>
		<div class="ex-design-frame-legend"><span style="--frame-color:var(--chart-1)">Input</span><span style="--frame-color:var(--chart-3)">Resolution</span><span style="--frame-color:var(--chart-2)">Action</span><span style="--frame-color:var(--ink-soft)">Plumbing</span></div>
		<div class="ex-design-execution-layout"><section class="ex-design-call-surface" aria-label="Calls in invocation order">${callStackMarkup(this.stack.frames, this.expandedSupport)}</section><section class="ex-design-frame-detail" aria-label="Call details"></section></div><ex-drawer class="ex-design-call-detail-sheet" label="Call details" size="content"><header class="ex-inspect-header"><strong>Call details</strong><button type="button" class="ex-design-call-detail-close" data-action="close-call-details" aria-label="Close call details" title="Close" autofocus>${icon('x')}</button></header><div class="ex-inspect-body ex-design-call-detail-body"></div></ex-drawer>`;
		this.querySelector('.ex-design-call-detail-sheet')?.addEventListener('close', (): void => {
			const detail = this.querySelector('.ex-design-call-detail-body > .ex-design-frame-detail');
			if (detail) this.querySelector('.ex-design-execution-layout')?.append(detail);
		});
		this.querySelector<SearchPicker>('ex-design-picker')?.configure({
			label: 'Choose a call stack',
			placeholder: 'Search call stacks…',
			icon: 'stack',
			selected: this.stackId,
			options: this.model.callStacks.map((stack) => ({
				value: stack.id,
				label: stack.title,
				detail: `${stack.frames.length} calls`,
			})),
		});
		this.updatePanels();
		restoreFocus();
	}
}
