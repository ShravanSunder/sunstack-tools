/** Keep native top-layer choices beside their invoker and within the available viewport. */
export function anchorPopover(
	button: HTMLButtonElement,
	panel: HTMLElement,
	signal: AbortSignal,
): void {
	const position = (): void => {
		const bounds = button.getBoundingClientRect();
		const viewportWidth = document.documentElement.clientWidth;
		const viewportHeight = document.documentElement.clientHeight;
		const edge = 8;
		const width = Math.min(320, viewportWidth - edge * 2);
		const below = viewportHeight - bounds.bottom - edge * 2;
		const above = bounds.top - edge * 2;
		const openAbove = below < 160 && above > below;
		panel.style.left = `${Math.max(edge, Math.min(bounds.left, viewportWidth - width - edge))}px`;
		panel.style.top = openAbove ? 'auto' : `${bounds.bottom + edge}px`;
		panel.style.bottom = openAbove ? `${viewportHeight - bounds.top + edge}px` : 'auto';
		panel.style.maxHeight = `min(75vh, ${Math.max(0, openAbove ? above : below)}px)`;
	};
	panel.addEventListener(
		'beforetoggle',
		(event: ToggleEvent): void => {
			if (event.newState === 'open') position();
		},
		{ signal },
	);
	const followInvoker = (): void => {
		if (panel.matches(':popover-open')) position();
	};
	window.addEventListener('resize', followInvoker, { signal });
	window.addEventListener('scroll', followInvoker, { signal, capture: true });
}
