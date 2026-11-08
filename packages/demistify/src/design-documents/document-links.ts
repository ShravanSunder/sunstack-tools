export type RequirementStage = 'specification' | 'program-design' | 'verification';

/** Agent-facing addresses use authored identity, without adding sharing controls to the document. */
export function requirementHref(id: string, stage?: RequirementStage): string {
	return `${stage ?? 'requirements'}.html?need=${encodeURIComponent(id)}${stage ? `#${stage}-${encodeURIComponent(id)}` : ''}`;
}
export function callHref(stackId: string, callId?: string): string {
	return `execution.html?stack=${encodeURIComponent(stackId)}${callId ? `&call=${encodeURIComponent(callId)}` : ''}`;
}

/** Requirement navigation records need and focused stage together. */
export function updateRequirementAddress(
	id: string,
	pushDetail = false,
	stage?: RequirementStage,
): void {
	const address = new URL(location.href);
	address.searchParams.set('need', id);
	address.searchParams.delete('stack');
	address.searchParams.delete('call');
	if (stage) address.searchParams.set('stage', stage);
	else address.searchParams.delete('stage');
	address.hash = stage ? `${stage}-${id}` : '';
	if (pushDetail) history.pushState({ ...history.state, labNeedDetail: true }, '', address);
	else history.replaceState(history.state, '', address);
}
export function updateRequirementListAddress(): void {
	const address = new URL(location.href);
	address.searchParams.delete('need');
	address.searchParams.delete('stage');
	address.hash = '';
	history.replaceState({ ...history.state, labNeedDetail: false }, '', address);
}
export function updateCallAddress(stackId: string, callId: string): void {
	const address = new URL(location.href);
	address.searchParams.delete('need');
	address.searchParams.set('stack', stackId);
	address.searchParams.set('call', callId);
	history.replaceState(history.state, '', address);
}
export function revealDocumentFragment(root: HTMLElement): void {
	if (!location.hash) return;
	requestAnimationFrame((): void => {
		let fragment: string;
		try {
			fragment = decodeURIComponent(location.hash.slice(1));
		} catch {
			return;
		}
		const target = document.getElementById(fragment);
		if (!target || !root.contains(target)) return;
		target.scrollIntoView({ block: 'start' });
	});
}
