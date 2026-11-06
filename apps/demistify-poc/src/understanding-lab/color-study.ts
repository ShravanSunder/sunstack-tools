import { icon, safeText } from './study-markup.js';
import { owners } from './study-model.js';

/** One explicit category map, rendered in two views; colors never infer an outcome. */
export class ColorStudy extends HTMLElement {
	private initialized = false;

	connectedCallback(): void {
		if (this.initialized) return;
		this.initialized = true;
		this.innerHTML = `<p class="lab-soft">Subtle card tint, using the same category hues as the diagram palette.</p>
		<div class="lab-category-legend">${owners.map((owner): string => `<span data-role="${owner.role}"><span class="lab-color-key" aria-hidden="true"></span>${owner.category}</span>`).join('')}</div>
		<div class="lab-color-comparison" data-treatment="tint">
			<div><h3>Relationship view</h3><div class="lab-owner-flow">${owners.map((owner, index): string => `${index ? '<span class="lab-inline-arrow" aria-hidden="true">→</span>' : ''}<button type="button" class="lab-owner-node" data-node="${owner.id}" data-role="${owner.role}" aria-pressed="${index === 0}"><span class="lab-node-heading">${icon(owner.icon)}<strong>${owner.name}</strong></span><span>${owner.description}</span><small>${owner.category}</small></button>`).join('')}</div><p class="lab-soft">Gateway uses Registry; Registry retains Session.</p></div>
			<div><h3>The same identities in a compact list</h3><div class="lab-owner-list">${owners.map((owner, index): string => `<button type="button" class="lab-owner-row" data-node="${owner.id}" data-role="${owner.role}" aria-pressed="${index === 0}">${icon(owner.icon)}<span><strong>${owner.name}</strong><small>${owner.category}</small></span><span class="lab-row-description">${owner.description}</span></button>`).join('')}</div></div>
		</div><section class="lab-selected-summary" aria-live="polite"><strong>Gateway · Boundary</strong><p>Selected in both views. Category color stays stable when selection changes.</p></section>
		<p class="lab-soft">Category is encoded by color and text. Selection uses an outline. Changed, failed and verified are separate status labels.</p>`;
		this.addEventListener('click', (event: MouseEvent): void => {
			if (!(event.target instanceof Element)) return;
			const id = event.target.closest<HTMLElement>('[data-node]')?.dataset['node'];
			const owner = owners.find((item): boolean => item.id === id);
			if (!owner) return;
			for (const node of this.querySelectorAll('[data-node]'))
				node.setAttribute('aria-pressed', String(node.getAttribute('data-node') === id));
			const summary = this.querySelector('.lab-selected-summary');
			if (summary)
				summary.innerHTML = `<strong>${safeText(owner.name)} · ${safeText(owner.category)}</strong><p>${safeText(owner.description)} Selected in both views; its category color is unchanged.</p>`;
		});
	}
}
