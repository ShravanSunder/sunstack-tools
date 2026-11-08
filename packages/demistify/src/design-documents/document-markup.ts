import type { ScenarioOutcome } from './collection.ts';

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

export function outcomeBadge(outcome: ScenarioOutcome, label?: string): string {
	const tone =
		outcome === 'Pass'
			? 'done'
			: outcome === 'Fail'
				? 'risk'
				: outcome === 'Partial'
					? 'question'
					: 'planned';
	return `<ex-badge tone="${tone}"${label ? ` aria-label="${safeText(label)}" title="${safeText(label)}"` : ''}>${outcome}</ex-badge>`;
}

/** Keep keyboard position when a study replaces its rendered content. */
export function restoreDocumentFocusAfterRender(element: HTMLElement): () => void {
	const active = document.activeElement;
	if (!(active instanceof HTMLButtonElement) || !element.contains(active)) return (): void => {};
	const attribute = [
		'data-choice',
		'data-capability',
		'data-need',
		'data-document',
		'data-frame',
		'data-action',
	].find((name): boolean => active.hasAttribute(name));
	if (!attribute) return (): void => {};
	const identity = active.getAttribute(attribute);
	return (): void => {
		const replacement = [...element.querySelectorAll<HTMLButtonElement>('button')].find(
			(button): boolean => button.getAttribute(attribute) === identity,
		);
		replacement?.focus({ preventScroll: true });
	};
}
