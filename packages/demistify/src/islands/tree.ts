import { createHead, readIcon } from '../core/head.ts';
import { createPhosphorIcon } from '../core/icons.ts';
import { reportIssue } from '../core/issues.ts';
import { readIslandSource } from '../core/source-text.ts';
import { createToneIcon, isTone, readTone, type Tone } from '../core/tones.ts';
import { revealElement } from '../layout/document.ts';

/**
 * A tree written as a Markdown list: a status report, a checklist, a breakdown or a map of a page.
 * A leading tone word (`done: Build passes`) gives a node its icon and colour; `[x]` and `[ ]` make
 * a checklist; a link to `#id` jumps to that island. `layout="map"` draws the same list as a
 * left-to-right mind map, which falls back to the outline when narrow.
 */
export class TreeIsland extends HTMLElement {
	private rendering = false;
	private bound = false;

	connectedCallback(): void {
		if (this.hasAttribute('data-inspector-content')) return;
		if (this.hasAttribute('prepared')) {
			this.bindControls();
			return;
		}
		if (this.rendering) return;
		this.rendering = true;
		void this.render();
	}

	private async render(): Promise<void> {
		const { renderMarkdownContent } = await import('../render/markdown-content.ts');
		const holder = document.createElement('div');
		holder.innerHTML = renderMarkdownContent(readIslandSource(this) ?? '');
		const list = holder.querySelector('ul, ol');
		if (!list) {
			reportIssue(this, 'ex-tree needs a Markdown list: "- item", children indented under it.');
			return;
		}
		const branches = convertList(list);
		const title = this.getAttribute('title');
		const body = document.createElement('div');
		body.className = 'ex-tree-body';
		if (title) {
			// The title is the root node, so a map grows from it.
			const root = document.createElement('ul');
			const item = document.createElement('li');
			item.className = 'ex-tree-root';
			const label = createHead('div', 'ex-tree-node', {
				tone: readTone(this, undefined),
				icon: readIcon(this),
				title,
			});
			item.append(label, branches);
			root.append(item);
			body.append(root);
			this.setAttribute('label', this.getAttribute('label') ?? title);
			this.removeAttribute('title');
		} else body.append(branches);
		this.replaceChildren(body);
		this.setAttribute('prepared', '');
		this.bindControls();
	}

	private bindControls(): void {
		if (this.bound) return;
		this.bound = true;
		this.addEventListener('click', (event): void => this.handleClick(event));
		this.addEventListener('keydown', (event): void => {
			if (event.key !== 'Enter' && event.key !== ' ') return;
			if (!(event.target instanceof HTMLElement) || !event.target.matches('[aria-expanded]'))
				return;
			event.preventDefault();
			toggle(event.target);
		});
	}

	private handleClick(event: MouseEvent): void {
		if (!(event.target instanceof Element)) return;
		const link = event.target.closest('a');
		if (link?.hash && link.origin === location.origin && link.pathname === location.pathname) {
			const target = document.getElementById(decodeURIComponent(link.hash.slice(1)));
			if (!target) return;
			event.preventDefault();
			revealElement(target);
			target.scrollIntoView({ behavior: 'smooth', block: 'center' });
			target.classList.add('ex-route-focus');
			window.setTimeout((): void => target.classList.remove('ex-route-focus'), 1600);
			return;
		}
		if (link) return;
		const node = event.target.closest('[aria-expanded]');
		if (node instanceof HTMLElement && this.contains(node)) toggle(node);
	}
}

function toggle(node: HTMLElement): void {
	const open = node.getAttribute('aria-expanded') !== 'true';
	node.setAttribute('aria-expanded', String(open));
	node.parentElement?.toggleAttribute('data-collapsed', !open);
}

/** The first text in a node: where a leading tone word sits. */
function firstText(element: Element): Text | undefined {
	const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
	for (let node = walker.nextNode(); node; node = walker.nextNode())
		if (node instanceof Text && node.data.trim()) return node;
	return undefined;
}

function convertList(list: Element): HTMLUListElement {
	const branches = document.createElement('ul');
	for (const item of Array.from(list.children)) {
		if (item.localName !== 'li') continue;
		const node = document.createElement('li');
		const label = document.createElement('div');
		label.className = 'ex-tree-node';
		const children: Element[] = [];
		for (const child of Array.from(item.childNodes)) {
			if (child instanceof Element && (child.localName === 'ul' || child.localName === 'ol'))
				children.push(child);
			else if (child instanceof Element && child.localName === 'p')
				label.append(...child.childNodes);
			else label.append(child);
		}
		let tone: Tone | undefined;
		const box = label.querySelector('input[type="checkbox"]');
		if (box instanceof HTMLInputElement) {
			tone = box.checked ? 'done' : 'planned';
			box.remove();
		}
		const text = firstText(label);
		const word = text ? /^\s*([a-z]+):\s*/.exec(text.data) : null;
		if (text && word?.[1] && isTone(word[1])) {
			tone = word[1];
			text.data = text.data.slice(word[0].length);
		}
		if (tone) {
			node.setAttribute('tone', tone);
			label.prepend(createToneIcon(tone));
		}
		node.append(label);
		const nested = children.map(convertList);
		if (nested.length > 0) {
			const count = nested.reduce((total, branch): number => total + branch.children.length, 0);
			label.setAttribute('aria-expanded', 'true');
			label.tabIndex = 0;
			const more = document.createElement('span');
			more.className = 'ex-tree-count';
			more.textContent = `+${count}`;
			label.append(createPhosphorIcon('caret-down'), more);
			node.append(...nested);
		}
		branches.append(node);
	}
	return branches;
}
