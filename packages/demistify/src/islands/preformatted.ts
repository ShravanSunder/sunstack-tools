import { readIslandSource } from '../core/source-text.ts';
import { installIslandInspector } from './inspector.ts';

/** Literal agent-authored text drawings; no terminal emulation or per-document styling. */
export class PreformattedIsland extends HTMLElement {
	connectedCallback(): void {
		if (!this.hasAttribute('prepared')) {
			const source = readIslandSource(this) ?? '';
			const pre = document.createElement('pre');
			pre.textContent = source;
			this.replaceChildren(pre);
			this.setAttribute('prepared', '');
		}
		if (!this.hasAttribute('zoom')) this.setAttribute('zoom', '');
		installIslandInspector(this);
	}
}
