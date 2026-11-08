/** Public native drawer behavior for typed HTML consumers. */
export interface DrawerElement extends HTMLElement {
	readonly open: boolean;
	show(): void;
	close(): void;
}
