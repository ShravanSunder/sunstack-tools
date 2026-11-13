export function safeText(value: string): string {
	return value
		.replaceAll('&', '&amp;')
		.replaceAll('<', '&lt;')
		.replaceAll('>', '&gt;')
		.replaceAll('"', '&quot;');
}

export function icon(name: string): string {
	return `<i class="ph ph-${safeText(name)}" aria-hidden="true"></i>`;
}

/** Keep keyboard position when a study replaces its rendered content. */
export function restoreDocumentFocusAfterRender(element: HTMLElement): () => void {
	const active = document.activeElement;
	if (!(active instanceof HTMLButtonElement) || !element.contains(active)) return (): void => {};
	const attribute = ['data-frame', 'data-action'].find((name): boolean =>
		active.hasAttribute(name),
	);
	if (!attribute) return (): void => {};
	const identity = active.getAttribute(attribute);
	return (): void => {
		const replacement = [...element.querySelectorAll<HTMLButtonElement>('button')].find(
			(button): boolean => button.getAttribute(attribute) === identity,
		);
		replacement?.focus({ preventScroll: true });
	};
}
