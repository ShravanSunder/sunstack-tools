import { classifyDocumentLink } from '../core/document-links.ts';
import { createIconButton } from '../core/icons.ts';
import { reportIssue } from '../core/issues.ts';
import { readSectionMembership } from '../core/section-membership.ts';
import { registerReaderControl } from '../layout/reader-controls.ts';
import { openDrawer } from './drawer.ts';

/** The authored tree stays live while the shared sidebar owns presentation and mutual exclusion. */
export class MapIsland extends HTMLElement {
	private root: HTMLElement | undefined;
	private tree: HTMLElement | undefined;
	private readonly header = document.createElement('header');
	private readonly body = document.createElement('div');
	private readonly trigger = createIconButton('map-trifold', 'Map', (): void => this.toggle());
	private closePanel: (() => void) | undefined;
	private unregisterControl: (() => void) | undefined;
	private treeObserver: MutationObserver | undefined;
	private membershipObserver: MutationObserver | undefined;
	private events: AbortController | undefined;
	private readonly handleShortcut = (event: KeyboardEvent): void => {
		if (
			event.defaultPrevented ||
			event.isComposing ||
			event.repeat ||
			event.metaKey ||
			event.ctrlKey ||
			event.altKey ||
			event.key.toLowerCase() !== 'm' ||
			(event.target instanceof Element &&
				event.target.closest(
					'input,textarea,select,[contenteditable]:not([contenteditable="false"]),[role="textbox"]',
				))
		)
			return;
		event.preventDefault();
		this.toggle();
	};
	private readonly followLink = (event: MouseEvent): void => {
		if (
			event.defaultPrevented ||
			event.button !== 0 ||
			event.metaKey ||
			event.ctrlKey ||
			event.altKey ||
			event.shiftKey ||
			!(event.target instanceof Element)
		)
			return;
		const link = event.target.closest('a');
		if (
			!(link instanceof HTMLAnchorElement) ||
			link.hasAttribute('download') ||
			(link.target && link.target.toLowerCase() !== '_self')
		)
			return;
		const destination = classifyDocumentLink(
			link.href,
			new URL(document.URL),
			new URL(document.baseURI),
		);
		if (
			destination.kind === 'local' &&
			document.getElementById(destination.reference) &&
			window.innerWidth < 900
		)
			this.closePanel?.();
	};

	connectedCallback(): void {
		if (this.closest('[data-inspector-content], [data-inspector-copy]')) return;
		if (!this.hasAttribute('prepared')) {
			const root = document.getElementById(this.getAttribute('for') ?? '');
			const tree = this.firstElementChild;
			if (
				!root ||
				root === this ||
				this.children.length !== 1 ||
				!(tree instanceof HTMLElement) ||
				tree.localName !== 'ex-tree'
			) {
				reportIssue(this, 'Map needs an existing scope id and exactly one authored ex-tree.');
				return;
			}
			if (document.querySelector('ex-map[prepared]')) {
				reportIssue(this, 'A page has at most one Map.');
				return;
			}
			this.root = root;
			this.tree = tree;
			this.header.className = 'ex-inspect-header';
			const title = document.createElement('strong');
			title.textContent = 'Map';
			this.header.append(
				title,
				createIconButton('x', 'Close Map', (): void => this.closePanel?.()),
			);
			this.body.className = 'ex-inspect-body ex-map-body';
			this.body.id = `ex-map-body-${crypto.randomUUID()}`;
			this.body.append(tree);
			this.append(this.header, this.body);
			this.trigger.classList.add('ex-map-trigger');
			this.trigger.setAttribute('aria-controls', this.body.id);
			this.trigger.setAttribute('aria-expanded', 'false');
			this.setAttribute('prepared', '');
		}
		if (!this.root || !this.tree) return;
		this.events?.abort();
		this.events = new AbortController();
		document.addEventListener('keydown', this.handleShortcut, { signal: this.events.signal });
		this.trigger.title = 'Show or hide Map (M)';
		this.trigger.setAttribute('aria-keyshortcuts', 'M');
		this.unregisterControl?.();
		this.unregisterControl = registerReaderControl('map', this.trigger);
		this.tree.addEventListener('click', this.followLink, true);
		this.treeObserver?.disconnect();
		this.treeObserver = new MutationObserver((): void => this.refreshCounts());
		this.treeObserver.observe(this.tree, {
			childList: true,
			subtree: true,
			attributes: true,
			attributeFilter: ['prepared'],
		});
		this.membershipObserver?.disconnect();
		this.membershipObserver = new MutationObserver((): void => this.refreshCounts());
		this.membershipObserver.observe(this.root, { childList: true, subtree: true });
		this.refreshCounts();
	}

	disconnectedCallback(): void {
		this.events?.abort();
		this.closePanel?.();
		this.unregisterControl?.();
		this.unregisterControl = undefined;
		this.tree?.removeEventListener('click', this.followLink, true);
		this.treeObserver?.disconnect();
		this.membershipObserver?.disconnect();
	}

	private toggle(): void {
		if (this.closePanel) {
			this.closePanel();
			return;
		}
		if (!this.root || !this.tree) return;
		this.closePanel = openDrawer('Map', () => ({
			presentation: { kind: 'map' },
			header: this.header,
			body: this.body,
			restore: (): void => {
				this.append(this.header, this.body);
				this.closePanel = undefined;
				this.trigger.setAttribute('aria-expanded', 'false');
			},
		}));
		this.trigger.setAttribute('aria-expanded', 'true');
	}

	private refreshCounts(): void {
		if (!this.root || !this.tree?.hasAttribute('prepared')) return;
		for (const link of this.tree.querySelectorAll('a.ex-tree-go')) {
			if (!(link instanceof HTMLAnchorElement)) continue;
			const destination = classifyDocumentLink(
				link.href,
				new URL(document.URL),
				new URL(document.baseURI),
			);
			if (destination.kind !== 'local') continue;
			const section = document.getElementById(destination.reference);
			if (!section || section.localName !== 'ex-section' || !this.root.contains(section)) continue;
			const label = link.closest('.ex-tree-node')?.querySelector('.ex-tree-label');
			if (!label) continue;
			let count = label.querySelector(':scope > .ex-map-count');
			if (!count) {
				count = document.createElement('span');
				count.className = 'ex-map-count';
				label.append(count);
			}
			const text = ` · ${readSectionMembership(section).total}`;
			if (count.textContent !== text) count.textContent = text;
		}
	}
}
