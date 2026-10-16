import { assignHeadingFragments } from '../core/heading-fragments.ts';
import { createIconButton } from '../core/icons.ts';
import { reportIssue } from '../core/issues.ts';
import {
	markdownSourceMarkup,
	parseMarkdownSource,
	type MarkdownSource,
} from '../core/markdown-source.ts';
import { readIslandSource } from '../core/source-text.ts';
import { installIslandInspector, openIsland } from './inspector.ts';

function retainedSource(owner: HTMLElement): MarkdownSource | undefined {
	const metadata = owner.querySelector(':scope > script[data-ex-markdown-source]');
	return metadata ? parseMarkdownSource(metadata.textContent ?? '') : undefined;
}

function installSourceInspection(owner: HTMLElement, source: MarkdownSource): void {
	if (
		owner.closest('[data-inspector-content], [data-inspector-copy]') ||
		owner.querySelector(':scope > .ex-inspect-controls [data-markdown-source]')
	)
		return;
	owner.classList.add('ex-inspectable');
	let controls = owner.querySelector<HTMLElement>(':scope > .ex-inspect-controls');
	if (!controls) {
		controls = document.createElement('span');
		controls.className = 'ex-inspect-controls';
		owner.append(controls);
	}
	const button = createIconButton('code', 'Show Markdown source', (): void => {
		const label =
			source.reference.length > 0
				? source.reference
				: (owner.getAttribute('label') ?? 'Markdown source');
		const sourceView = document.createElement('div');
		sourceView.setAttribute('label', label);
		const pre = document.createElement('pre');
		const code = document.createElement('code');
		code.textContent = source.text;
		pre.append(code);
		sourceView.append(pre);
		openIsland(sourceView, 'sidebar');
	});
	button.dataset['markdownSource'] = '';
	button.classList.add('ex-inspect-trigger');
	controls.append(button);
}

/** Inline or fetched Markdown; retained source never replaces its rendered document view. */
export class MarkdownIsland extends HTMLElement {
	private renderPromise: Promise<void> | undefined;
	connectedCallback(): void {
		if (this.hasAttribute('prepared')) {
			assignHeadingFragments(this);
			installIslandInspector(this);
			const source = retainedSource(this);
			if (source) installSourceInspection(this, source);
			return;
		}
		this.renderPromise ??= this.renderContent();
	}
	private async readSource(): Promise<MarkdownSource | undefined> {
		const embedded = retainedSource(this);
		if (embedded) return embedded;
		const reference = this.getAttribute('src');
		if (reference !== null) {
			if (!reference.trim()) throw new Error('Markdown src must name a file.');
			this.setAttribute('aria-busy', 'true');
			this.textContent = 'Loading Markdown…';
			const response = await fetch(new URL(reference, this.baseURI));
			if (!response.ok) throw new Error(`HTTP ${response.status} loading ${reference}.`);
			return { text: await response.text(), reference };
		}
		const text = readIslandSource(this);
		return text === undefined ? undefined : { text, reference: '' };
	}
	private async renderContent(): Promise<void> {
		try {
			const source = await this.readSource();
			if (source) {
				const { renderMarkdownContent } = await import('../render/markdown-content.ts');
				this.innerHTML = renderMarkdownContent(source.text) + markdownSourceMarkup(source);
				// A Markdown file's relative links and pictures belong to that file's directory.
				if (source.reference) {
					const base = new URL(source.reference, this.baseURI);
					for (const element of this.querySelectorAll<HTMLAnchorElement | HTMLImageElement>(
						'a[href], img[src]',
					)) {
						const attribute = element instanceof HTMLAnchorElement ? 'href' : 'src';
						const value = element.getAttribute(attribute);
						if (value && !value.startsWith('#'))
							element.setAttribute(attribute, new URL(value, base).href);
					}
				}
			}
			this.setAttribute('prepared', '');
			assignHeadingFragments(this);
			installIslandInspector(this);
			if (source) installSourceInspection(this, source);
			this.dispatchEvent(new Event('island-ready', { bubbles: true }));
		} catch (error) {
			const localHint =
				location.protocol === 'file:'
					? ' Embed Markdown files before opening this HTML, or serve it over HTTP; browsers block sibling-file fetches.'
					: '';
			this.textContent = `Markdown could not load: ${String(error)}${localHint}`;
			this.setAttribute('role', 'alert');
			reportIssue(this, this.textContent);
		} finally {
			this.removeAttribute('aria-busy');
		}
	}
}
