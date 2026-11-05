import type { RequirementExample } from './study-model.js';

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

export function outcomeBadge(outcome: RequirementExample['outcome'], label?: string): string {
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

/** The same chain is used beside a requirement and beside the frame that serves it. */
export function coverageChain(
	requirement: RequirementExample,
	includeNeed: boolean = true,
): string {
	return `<div class="lab-coverage-chain" aria-label="Need to observation">
		${includeNeed ? `<section data-stage="need"><small>Requirement · ${requirement.id}</small><strong>${safeText(requirement.title)}</strong><p>${safeText(requirement.need)}</p></section>` : ''}
		<section data-stage="rule" id="specification-${requirement.id}"><small>Specification</small><strong>${safeText(requirement.obligation)}</strong></section>
		<section data-stage="owner" id="program-design-${requirement.id}"><small>Program design</small><strong>${safeText(requirement.owner)}</strong></section>
		<section data-stage="proof" id="verification-${requirement.id}"><small>Verification</small><strong>${safeText(requirement.proof)}</strong>${outcomeBadge(requirement.outcome)}<p>${safeText(requirement.observation)}</p></section>
	</div>`;
}

export function bindStudyChoices(element: HTMLElement, select: (value: string) => void): void {
	element.addEventListener('click', (event: MouseEvent): void => {
		if (!(event.target instanceof Element)) return;
		const button = event.target.closest<HTMLButtonElement>('button[data-choice]');
		if (button?.dataset['choice']) select(button.dataset['choice']);
	});
}

/** Keep keyboard position when a study replaces its rendered content. */
export function restoreStudyFocusAfterRender(element: HTMLElement): () => void {
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
