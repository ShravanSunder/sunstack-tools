import { createIconButton, createPhosphorIcon } from './icon-controls.ts';

export class HeadingNavigation extends HTMLElement {
	private contentObserver: MutationObserver | undefined;
	private scheduledFrame: number | undefined;
	private navigationPanel: HTMLElement | undefined;
	private railMarkers: HTMLElement | undefined;
	private contentRoot: HTMLElement | undefined;
	private pageLinks: readonly HTMLAnchorElement[] = [];
	private readonly headingLinks = new Map<Element, readonly HTMLAnchorElement[]>();
	private readonly sidebarMedia = window.matchMedia('(min-width: 1440px)');
	private changingPopover = false;
	private readonly updatePresentation = (): void => {
		if (!this.navigationPanel) return;
		if (this.navigationPanel.matches(':popover-open')) this.closePanel();
		this.toggleAttribute('data-pinned', this.sidebarMedia.matches);
		this.navigationPanel.toggleAttribute('data-pinned', this.sidebarMedia.matches);
		if (this.sidebarMedia.matches) this.navigationPanel.removeAttribute('popover');
		else this.navigationPanel.popover = 'auto';
	};
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
			this.connectObservers();
			return;
		}
		this.contentRoot =
			(this.getAttribute('for') ? document.getElementById(this.getAttribute('for') ?? '') : null) ??
			document.querySelector<HTMLElement>('ex-document, main') ??
			document.body;
		this.pageLinks = Array.from(this.querySelectorAll<HTMLAnchorElement>('a[href]'));
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
		this.replaceChildren(menuButton, railMarkers);
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
		this.contentObserver.observe(this.contentRoot, {
			subtree: true,
			childList: true,
			attributes: true,
			attributeFilter: ['hidden'],
		});
		window.addEventListener('scroll', this.scheduleCurrentSection, { passive: true });
		window.addEventListener('resize', this.scheduleCurrentSection);
		this.sidebarMedia.addEventListener('change', this.updatePresentation);
		this.updatePresentation();
		this.renderHeadings();
	}

	disconnectedCallback(): void {
		this.contentObserver?.disconnect();
		window.removeEventListener('scroll', this.scheduleCurrentSection);
		window.removeEventListener('resize', this.scheduleCurrentSection);
		this.sidebarMedia.removeEventListener('change', this.updatePresentation);
		this.navigationPanel?.remove();
		if (this.scheduledFrame !== undefined) cancelAnimationFrame(this.scheduledFrame);
		this.scheduledFrame = undefined;
		this.changingPopover = false;
	}

	private renderHeadings(): void {
		if (!this.contentRoot || !this.navigationPanel || !this.railMarkers) return;
		const headings = Array.from(
			this.contentRoot.querySelectorAll<HTMLHeadingElement>('h1,h2,h3,h4,h5,h6'),
		).filter((heading): boolean => !heading.closest('[hidden]'));
		this.headingLinks.clear();
		this.navigationPanel.replaceChildren();
		this.railMarkers.replaceChildren();
		const outline = document.createElement('div');
		outline.className = 'ex-heading-outline';
		const sectionLabel = document.createElement('strong');
		sectionLabel.className = 'ex-navigation-label';
		sectionLabel.textContent = 'On this page';
		outline.append(sectionLabel);
		this.navigationPanel.append(outline);
		for (const heading of headings) {
			if (!heading.id) {
				const slug =
					heading.textContent
						?.trim()
						.toLowerCase()
						.replace(/[^\p{L}\p{N}]+/gu, '-')
						.replace(/^-|-$/g, '') || 'section';
				const baseId = `heading-${slug}`;
				let suffix = 1;
				heading.id = baseId;
				while (document.getElementById(heading.id) !== heading)
					heading.id = `${baseId}-${++suffix}`;
			}
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
		if (this.pageLinks.length) {
			const pages = document.createElement('div');
			pages.className = 'ex-heading-pages';
			const pagesLabel = document.createElement('strong');
			pagesLabel.className = 'ex-navigation-label';
			pagesLabel.textContent = 'Other pages';
			pages.append(pagesLabel);
			for (const link of this.pageLinks) {
				link.classList.add('ex-page-link');
				if (!link.querySelector('.ph')) link.append(createPhosphorIcon('arrow-up-right'));
				pages.append(link);
			}
			this.navigationPanel.append(pages);
		}
		this.updateCurrentSection();
	}

	private containsNavigation(target: EventTarget | null): boolean {
		return (
			target instanceof Node &&
			(this.contains(target) || Boolean(this.navigationPanel?.contains(target)))
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

	private updateCurrentSection(): void {
		let activeHeading = this.headingLinks.keys().next().value;
		let latestTop = -Infinity;
		for (const heading of this.headingLinks.keys()) {
			const top = heading.getBoundingClientRect().top;
			if (top <= Math.max(64, window.innerHeight * 0.25) && top > latestTop) {
				latestTop = top;
				activeHeading = heading;
			}
		}
		for (const [heading, links] of this.headingLinks) {
			for (const link of links) {
				if (heading === activeHeading) link.setAttribute('aria-current', 'location');
				else link.removeAttribute('aria-current');
			}
		}
	}
}
