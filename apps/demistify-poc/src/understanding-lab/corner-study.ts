import { icon } from './study-markup.js';

/** Identical cards hold the palette, content and elevation constant while one surface changes. */
export class CornerStudy extends HTMLElement {
	private initialized = false;

	connectedCallback(): void {
		if (this.initialized) return;
		this.initialized = true;
		const variants = [
			{ id: 'uniform', label: 'Uniform', change: 'One surface throughout' },
			{ id: 'bands', label: 'Darker header/footer', change: 'Only the two bands change' },
			{ id: 'body', label: 'Lighter body', change: 'Only the body changes' },
		];
		this.innerHTML = `<p class="lab-soft">Same card, content, base color, radius and shadow. Only the named surfaces change.</p><div class="lab-study-toolbar"><label class="lab-elevation-control"><input type="checkbox" data-elevation /> Stronger outer shadow on all three</label></div>
		<div class="lab-corner-samples" data-elevation="shared" style="--study-radius:var(--radius-md)">${variants.map((variant): string => `<figure class="lab-card-comparison" data-surface="${variant.id}"><figcaption><strong>${variant.label}</strong><small>${variant.change}</small></figcaption><section class="lab-container-sample"><header>${icon('globe')}<div class="lab-card-title" role="heading" aria-level="3">Gateway</div></header><p>Routes reconnect requests.</p><div class="lab-sample-foot">Selected node details</div></section></figure>`).join('')}</div><p class="lab-current-value">8 px corners throughout. One shared outer shadow; no header shadow.</p><p class="lab-light-surface-note lab-soft">In the light theme, the base surface is already white, so the lighter-body treatment reaches the same white limit.</p>`;
		this.addEventListener('change', (event): void => {
			if (event.target instanceof HTMLInputElement && event.target.hasAttribute('data-elevation'))
				this.querySelector('.lab-corner-samples')?.setAttribute(
					'data-elevation',
					event.target.checked ? 'strong' : 'shared',
				);
		});
	}
}
