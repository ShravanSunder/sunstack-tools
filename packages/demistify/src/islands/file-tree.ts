import { normalizeIslandText } from '../core/source-text.ts';
import { installIslandInspector } from './inspector.ts';

/** A plain-text tree of files a change touches. */
export class FileTreeIsland extends HTMLElement {
	connectedCallback(): void {
		if (this.hasAttribute('prepared')) {
			installIslandInspector(this);
			return;
		}
		this.classList.add('ex-file-tree');
		this.textContent = normalizeIslandText(
			this.querySelector('script[type="text/plain"]')?.textContent ?? this.textContent ?? '',
		);
		this.setAttribute('prepared', '');
		installIslandInspector(this);
	}
}
