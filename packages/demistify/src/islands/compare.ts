import { installIslandInspector } from './inspector.ts';

/**
 * Two views of one thing: before and after. Side by side by default; with `slider` the two overlay
 * and a divider (drag it, or use the arrow keys) reveals one over the other, which suits screenshots
 * and generated pictures of the same size. A child's `label` names its side.
 */
export class CompareIsland extends HTMLElement {
	connectedCallback(): void {
		if (this.hasAttribute('data-inspector-content')) return;
		this.classList.add('ex-compare');
		if (!this.hasAttribute('prepared')) {
			// A bare picture gets a frame, so its side can carry a label.
			const sides = Array.from(this.children)
				.filter((child): child is HTMLElement => child instanceof HTMLElement)
				.map((child): HTMLElement => {
					if (child.localName !== 'img') return child;
					const frame = document.createElement('div');
					const label = child.getAttribute('label');
					if (label) frame.setAttribute('label', label);
					child.before(frame);
					frame.append(child);
					return frame;
				});
			sides.forEach((side, index): void => {
				side.dataset['side'] =
					side.getAttribute('label') ??
					side.getAttribute('slot')?.replace(/^\w/, (letter): string => letter.toUpperCase()) ??
					(index === 0 ? 'Before' : 'After');
			});
			if (this.hasAttribute('slider') && sides.length === 2) this.installSlider();
			this.setAttribute('prepared', '');
		}
		installIslandInspector(this);
	}

	private installSlider(): void {
		const range = document.createElement('input');
		range.type = 'range';
		range.min = '0';
		range.max = '100';
		range.value = '50';
		range.className = 'ex-compare-range';
		range.setAttribute('aria-label', 'Reveal before and after');
		const update = (): void => this.style.setProperty('--split', `${range.value}%`);
		range.addEventListener('input', update);
		update();
		this.append(range);
	}
}

export class DiagramDiffIsland extends CompareIsland {}
