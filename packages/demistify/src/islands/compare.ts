import { ComparisonViews } from './comparison-views.ts';
import { installIslandInspector } from './inspector.ts';

/**
 * Named alternatives use shared topic rows or authored content tabs. The image-only `slider`
 * overlays two same-size pictures with a native range reveal. A child's `label` names its view.
 */
export class CompareIsland extends HTMLElement {
	private sliderBound = false;
	private comparison: ComparisonViews | undefined;
	connectedCallback(): void {
		if (this.hasAttribute('data-inspector-content')) return;
		this.classList.add('ex-compare');
		if (!this.hasAttribute('slider')) {
			if (!this.comparison) {
				const sides = Array.from(this.children).filter(
					(child): child is HTMLElement =>
						child instanceof HTMLElement && !child.classList.contains('ex-inspect-controls'),
				);
				for (const side of sides)
					if (side.localName === 'img') {
						const frame = document.createElement('div');
						const label = side.getAttribute('label');
						if (label) frame.setAttribute('label', label);
						side.before(frame);
						frame.append(side);
						sides[sides.indexOf(side)] = frame;
					}
				this.comparison = new ComparisonViews(this, sides);
				this.setAttribute('prepared', '');
			}
			this.comparison.connect();
			installIslandInspector(this);
			return;
		}
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
			if (this.hasAttribute('slider') && sides.length === 2) this.installSlider(sides);
			this.setAttribute('prepared', '');
		}
		const range = this.querySelector<HTMLInputElement>(':scope > .ex-compare-range');
		if (range && !this.sliderBound)
			this.installSlider(
				Array.from(this.querySelectorAll<HTMLElement>(':scope > [data-side]')),
				range,
			);
		installIslandInspector(this);
	}
	disconnectedCallback(): void {
		this.comparison?.disconnect();
	}

	private installSlider(
		sides: readonly HTMLElement[],
		range = document.createElement('input'),
	): void {
		this.sliderBound = true;
		const value = range.value;
		range.type = 'range';
		range.min = '0';
		range.max = '100';
		range.value = range.isConnected || range.hasAttribute('class') ? value : '50';
		range.className = 'ex-compare-range';
		range.setAttribute('aria-label', 'Reveal before and after');
		// The split sits under the thumb's centre, which travels the width less one thumb. It is set on
		// the parts that move, so it survives the island opening in the side panel or large view.
		const parts = [range, ...sides];
		const update = (): void => {
			const split = `calc(var(--thumb) / 2 + (100% - var(--thumb)) * ${Number(range.value) / 100})`;
			for (const part of parts) part.style.setProperty('--split', split);
		};
		range.addEventListener('input', update);
		update();
		this.append(range);
	}
}

export class DiagramDiffIsland extends CompareIsland {}
