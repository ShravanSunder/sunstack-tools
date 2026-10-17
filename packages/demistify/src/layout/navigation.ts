import { assignHeadingFragments } from '../core/heading-fragments.ts';
import { createIconButton } from '../core/icons.ts';
import { createDocumentToolbar, type DocumentToolbar } from './page-navigation.ts';
import { createPresentationMenu } from './presentations.ts';

export class HeadingNavigation extends HTMLElement {
	private contentObserver: MutationObserver | undefined;
	private scheduledFrame: number | undefined;
	private navigationPanel: HTMLElement | undefined;
	private railMarkers: HTMLElement | undefined;
	private contentRoot: HTMLElement | undefined;
	private pageLinks: readonly HTMLAnchorElement[] = [];
	private presentationsButton: HTMLButtonElement | undefined;
	private documentToolbar: DocumentToolbar | undefined;
	private headingButton: HTMLButtonElement | undefined;
	private readonly headingLinks = new Map<Element, readonly HTMLAnchorElement[]>();
	private readonly sidebarMedia = window.matchMedia('(min-width: 1440px)');
	private changingPopover = false;
	private readonly updatePresentation = (): void => {
		if (!this.navigationPanel) return;
		if (this.navigationPanel.matches(':popover-open')) this.closePanel();
		this.toggleAttribute('data-pinned', this.sidebarMedia.matches);
		this.navigationPanel.toggleAttribute('data-pinned', this.sidebarMedia.matches);
		if (this.headingButton)
			this.headingButton.hidden =
				this.sidebarMedia.matches || this.navigationPanel.hasAttribute('data-empty');
		if (this.sidebarMedia.matches) this.navigationPanel.removeAttribute('popover');
		else this.navigationPanel.popover = 'auto';
	};
	private currentHeading: Element | undefined;
	private requestedHeading: Element | undefined;
	private readerEvents: AbortController | undefined;
	private touchY = 0;
	private readonly requestHeading = (event: MouseEvent): void => {
		if (!(event.target instanceof Element) || event.metaKey || event.ctrlKey || event.shiftKey)
			return;
		const href = event.target.closest('a')?.getAttribute('href');
		if (!href?.startsWith('#')) return;
		let fragment: string;
		try {
			fragment = decodeURIComponent(href.slice(1));
		} catch {
			return;
		}
		const target = document.getElementById(fragment);
		if (!target || !this.headingLinks.has(target)) return;
		this.requestedHeading = target;
		this.updateCurrentSection();
	};
	private releaseHeading(target: EventTarget | null, direction: number): void {
		if (!this.requestedHeading) return;
		for (
			let element = target instanceof Element ? target : null;
			element && element !== document.body;
			element = element.parentElement
		) {
			const style = getComputedStyle(element);
			if (!/(auto|scroll)/.test(style.overflowY) || element.scrollHeight <= element.clientHeight)
				continue;
			const canScroll =
				direction < 0
					? element.scrollTop > 0
					: element.scrollTop + element.clientHeight < element.scrollHeight - 1;
			if (canScroll || /contain|none/.test(style.overscrollBehaviorY)) return;
		}
		this.requestedHeading = undefined;
		this.scheduleCurrentSection();
	}
	/** Whether each heading's top is above the reading line (a quarter down the viewport). */
	private readonly passedHeadings = new WeakMap<Element, boolean>();
	private headingObserver: IntersectionObserver | undefined;
	private readonly scheduleCurrentSection = (): void => {
		if (this.scheduledFrame !== undefined) return;
		this.scheduledFrame = requestAnimationFrame((): void => {
			this.scheduledFrame = undefined;
			this.updateCurrentSection();
		});
	};

	connectedCallback(): void {
		if (this.navigationPanel) {
			document.body.append(this.navigationPanel);
			if (this.documentToolbar)
				document.body.append(this.documentToolbar.element, this.documentToolbar.pagePanel);
			if (this.presentationsButton) document.body.append(this.presentationsButton);
			this.connectObservers();
			return;
		}
		this.contentRoot =
			(this.getAttribute('for') ? document.getElementById(this.getAttribute('for') ?? '') : null) ??
			document.querySelector<HTMLElement>('ex-document, main') ??
			document.body;
		this.pageLinks = Array.from(this.querySelectorAll<HTMLAnchorElement>('a[href]')).filter(
			(link): boolean => !link.closest('[slot="presentations"]'),
		);
		this.presentationsButton = createPresentationMenu(this)?.button;
		if (this.presentationsButton) document.body.append(this.presentationsButton);
		this.documentToolbar = createDocumentToolbar(this.pageLinks);
		const navigationPanel = document.createElement('nav');
		navigationPanel.className = 'ex-heading-panel';
		navigationPanel.popover = 'auto';
		navigationPanel.setAttribute('aria-label', 'Page navigation');
		this.navigationPanel = navigationPanel;
		const railMarkers = document.createElement('nav');
		railMarkers.className = 'ex-heading-markers';
		railMarkers.setAttribute('aria-label', 'Section shortcuts');
		this.railMarkers = railMarkers;
		const menuButton = createIconButton('list', 'Browse page headings', (): void =>
			this.openPanel(),
		);
		// Native invoker ownership prevents a hover-opened popover dismissing on its own button.
		menuButton.popoverTargetElement = navigationPanel;
		menuButton.popoverTargetAction = 'show';
		this.headingButton = menuButton;
		this.documentToolbar.element.prepend(menuButton);
		menuButton.addEventListener('pointerenter', (): void => this.openPanel());
		this.replaceChildren(railMarkers);
		document.body.append(navigationPanel);
		this.addEventListener('pointerenter', (): void => this.openPanel());
		this.addEventListener('pointerleave', (event: PointerEvent): void => {
			if (!this.containsNavigation(event.relatedTarget)) this.closePanel();
		});
		this.addEventListener('focusout', (event: FocusEvent): void => {
			if (!this.containsNavigation(event.relatedTarget)) this.closePanel();
		});
		railMarkers.addEventListener('pointerover', (event: PointerEvent): void => {
			if (event.target instanceof Element) this.previewHeading(event.target.closest('a'));
		});
		railMarkers.addEventListener('focusin', (event: FocusEvent): void => {
			if (event.target instanceof HTMLAnchorElement) {
				this.openPanel();
				this.previewHeading(event.target);
			}
		});
		railMarkers.addEventListener('pointerleave', (): void => this.previewHeading(null));
		navigationPanel.addEventListener('pointerleave', (event: PointerEvent): void => {
			if (!this.containsNavigation(event.relatedTarget)) this.closePanel();
		});
		navigationPanel.addEventListener('click', (event: MouseEvent): void => {
			if (event.target instanceof Element && event.target.closest('a')) this.closePanel();
		});
		navigationPanel.addEventListener('focusout', (event: FocusEvent): void => {
			if (!this.containsNavigation(event.relatedTarget)) this.closePanel();
		});
		navigationPanel.addEventListener('beforetoggle', (): void => {
			this.changingPopover = true;
		});
		navigationPanel.addEventListener('toggle', (): void => {
			this.changingPopover = false;
			if (
				!this.matches(':hover') &&
				!this.headingButton?.matches(':hover') &&
				!navigationPanel.matches(':hover') &&
				!this.containsNavigation(document.activeElement)
			)
				this.closePanel();
		});
		this.contentObserver = new MutationObserver((records): void => {
			// The body fallback includes our portal: its outline writes are not content changes.
			if (
				records.some(
					(record): boolean =>
						Boolean(this.contentRoot?.contains(record.target)) &&
						!this.containsNavigation(record.target),
				)
			)
				this.renderHeadings();
		});
		this.connectObservers();
	}

	private connectObservers(): void {
		if (!this.contentRoot || !this.contentObserver) return;
		this.readerEvents?.abort();
		this.readerEvents = new AbortController();
		const { signal } = this.readerEvents;
		document.addEventListener('click', this.requestHeading, { signal });
		// A large jump can skip the observer's narrow reading window entirely. Reconcile once
		// when native page scrolling settles, rather than measuring every heading every frame.
		document.addEventListener(
			'scrollend',
			(event): void => {
				if (event.target !== document) return;
				const line = window.innerHeight * 0.25;
				for (const heading of this.headingLinks.keys()) {
					const bounds = heading.getBoundingClientRect();
					this.passedHeadings.set(heading, bounds.height > 0 && bounds.top <= line);
				}
				if (this.requestedHeading) {
					const bounds = this.requestedHeading.getBoundingClientRect();
					const margin =
						Number.parseFloat(getComputedStyle(this.requestedHeading).scrollMarginTop) || 0;
					const maximum = document.documentElement.scrollHeight - window.innerHeight;
					const requestedPosition = Math.max(
						0,
						Math.min(maximum, window.scrollY + bounds.top - margin),
					);
					// Scrollbar movement has no wheel/key event. Keep the override only at its actual
					// native anchor position (including bottom clamp), after smooth scrolling settles.
					if (Math.abs(window.scrollY - requestedPosition) > 2) this.requestedHeading = undefined;
				}
				this.updateCurrentSection();
			},
			{ signal },
		);
		document.addEventListener(
			'wheel',
			(event): void => {
				if (!event.defaultPrevented && event.deltaY)
					this.releaseHeading(event.target, event.deltaY);
			},
			{ signal, passive: true },
		);
		document.addEventListener(
			'touchstart',
			(event): void => {
				this.touchY = event.touches[0]?.clientY ?? 0;
			},
			{ signal, passive: true },
		);
		document.addEventListener(
			'touchmove',
			(event): void => {
				const nextY = event.touches[0]?.clientY ?? this.touchY;
				if (!event.defaultPrevented && nextY !== this.touchY)
					this.releaseHeading(event.target, this.touchY - nextY);
				this.touchY = nextY;
			},
			{ signal, passive: true },
		);
		document.addEventListener(
			'keydown',
			(event): void => {
				if (
					event.defaultPrevented ||
					event.altKey ||
					event.ctrlKey ||
					event.metaKey ||
					(event.target instanceof Element &&
						event.target.closest(
							'input,textarea,select,[contenteditable="true"],[role="tab"],[role="radio"]',
						))
				)
					return;
				if (
					event.key === ' ' &&
					event.target instanceof Element &&
					event.target.closest('button,a')
				)
					return;
				if (['ArrowUp', 'ArrowDown', 'PageUp', 'PageDown', 'Home', 'End', ' '].includes(event.key))
					this.releaseHeading(
						event.target,
						['ArrowUp', 'PageUp', 'Home'].includes(event.key) ||
							(event.key === ' ' && event.shiftKey)
							? -1
							: 1,
					);
			},
			{ signal },
		);
		this.contentObserver.observe(this.contentRoot, {
			subtree: true,
			childList: true,
			attributes: true,
			attributeFilter: ['hidden'],
		});
		this.sidebarMedia.addEventListener('change', this.updatePresentation);
		this.updatePresentation();
		this.renderHeadings();
	}

	disconnectedCallback(): void {
		this.readerEvents?.abort();
		this.contentObserver?.disconnect();
		this.headingObserver?.disconnect();
		this.sidebarMedia.removeEventListener('change', this.updatePresentation);
		this.navigationPanel?.remove();
		this.documentToolbar?.element.remove();
		this.documentToolbar?.pagePanel.remove();
		this.presentationsButton?.remove();
		if (this.scheduledFrame !== undefined) cancelAnimationFrame(this.scheduledFrame);
		this.scheduledFrame = undefined;
		this.changingPopover = false;
	}

	private renderHeadings(): void {
		if (!this.contentRoot || !this.navigationPanel || !this.railMarkers) return;
		assignHeadingFragments(this.contentRoot);
		const firstHeading = this.contentRoot.querySelector('h1');
		const headings = Array.from(
			this.contentRoot.querySelectorAll<HTMLHeadingElement>('h1,h2,h3,h4,h5,h6'),
		).filter(
			(heading): boolean =>
				heading !== firstHeading && !heading.closest('[hidden], figcaption, ex-gallery'),
		);
		document.documentElement.dataset['outline'] = String(headings.length > 0);
		this.navigationPanel.toggleAttribute('data-empty', headings.length === 0);
		if (this.headingButton)
			this.headingButton.hidden = this.sidebarMedia.matches || headings.length === 0;
		this.headingLinks.clear();
		this.navigationPanel.replaceChildren();
		this.railMarkers.replaceChildren();
		const outline = document.createElement('div');
		outline.className = 'ex-heading-outline';
		const sectionLabel = document.createElement('strong');
		sectionLabel.className = 'ex-navigation-label';
		sectionLabel.textContent = this.pageName();
		this.documentToolbar?.updateTitle(this.pageName());
		outline.append(sectionLabel);
		this.navigationPanel.append(outline);
		for (const heading of headings) {
			const label = heading.textContent?.trim() ?? '';
			const level = heading.tagName.slice(1);
			const marker = document.createElement('a');
			marker.href = `#${heading.id}`;
			marker.dataset['level'] = level;
			marker.setAttribute('aria-label', label);
			const menuLink = marker.cloneNode(false);
			if (!(menuLink instanceof HTMLAnchorElement)) continue;
			menuLink.textContent = label;
			this.headingLinks.set(heading, [marker, menuLink]);
			this.railMarkers.append(marker);
			outline.append(menuLink);
		}
		this.currentHeading = undefined;
		this.observeHeadings();
		this.updateCurrentSection();
	}

	private pageName(): string {
		const pageTitle = this.pageLinks
			.find((link): boolean => new URL(link.href).pathname === location.pathname)
			?.textContent?.trim();
		if (pageTitle) return pageTitle;
		const heading = this.contentRoot?.querySelector('h1')?.textContent?.trim();
		return heading?.length ? heading : document.title;
	}

	private containsNavigation(target: EventTarget | null): boolean {
		return (
			target instanceof Node &&
			(this.contains(target) ||
				Boolean(this.headingButton?.contains(target)) ||
				Boolean(this.navigationPanel?.contains(target)))
		);
	}

	private openPanel(): void {
		if (
			!this.navigationPanel?.hasAttribute('popover') ||
			this.changingPopover ||
			this.navigationPanel.matches(':popover-open')
		)
			return;
		this.navigationPanel.showPopover();
	}

	private closePanel(): void {
		if (this.changingPopover || !this.navigationPanel?.matches(':popover-open')) return;
		this.navigationPanel.hidePopover();
	}

	private previewHeading(marker: HTMLAnchorElement | null): void {
		for (const links of this.headingLinks.values()) {
			for (const link of links) link.toggleAttribute('data-preview', links[0] === marker);
			if (links[0] === marker) links[1]?.scrollIntoView({ block: 'nearest' });
		}
	}

	/**
	 * The browser reports when a heading crosses the reading line, so scrolling reads no layout:
	 * measuring every heading on each scroll frame forced a full layout while islands rendered.
	 */
	private observeHeadings(): void {
		this.headingObserver?.disconnect();
		this.headingObserver = new IntersectionObserver(
			(entries): void => {
				for (const entry of entries) {
					const line = entry.rootBounds?.bottom ?? window.innerHeight * 0.25;
					const bounds = entry.boundingClientRect;
					this.passedHeadings.set(entry.target, bounds.height > 0 && bounds.top <= line);
				}
				this.scheduleCurrentSection();
			},
			{ rootMargin: '0px 0px -75% 0px' },
		);
		for (const heading of this.headingLinks.keys()) this.headingObserver.observe(heading);
	}

	private updateCurrentSection(): void {
		let activeHeading = this.headingLinks.keys().next().value;
		for (const heading of this.headingLinks.keys())
			if (this.passedHeadings.get(heading)) activeHeading = heading;
		if (this.requestedHeading && this.headingLinks.has(this.requestedHeading))
			activeHeading = this.requestedHeading;
		// Rewriting every link on every scroll frame restyles the outline; only touch it on a change.
		if (activeHeading === this.currentHeading) return;
		this.currentHeading = activeHeading;
		for (const [heading, links] of this.headingLinks) {
			for (const link of links) {
				if (heading === activeHeading) link.setAttribute('aria-current', 'location');
				else link.removeAttribute('aria-current');
			}
		}
	}
}
