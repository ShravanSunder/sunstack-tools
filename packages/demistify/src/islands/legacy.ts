import { installIslandInspector } from './inspector.ts';

/* Earlier lab vocabulary, kept for the comparison pages and not taught in the authoring guide. */
export class DecisionIsland extends HTMLElement {
	connectedCallback(): void {
		this.classList.add('decision');
		installIslandInspector(this);
	}
}
export class CompareIsland extends HTMLElement {
	connectedCallback(): void {
		this.classList.add('ex-compare');
		installIslandInspector(this);
	}
}
export class TimelineIsland extends HTMLElement {
	connectedCallback(): void {
		this.classList.add('ex-timeline');
		installIslandInspector(this);
	}
}
export class LayerIsland extends HTMLElement {
	connectedCallback(): void {
		this.classList.add('band');
		installIslandInspector(this);
	}
}
export class DiagramDiffIsland extends CompareIsland {}
