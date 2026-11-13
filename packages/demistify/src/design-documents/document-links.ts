import { readCollection } from './collection.ts';

/** Need destinations use the original authored body rather than a generated reader. */
export function needHref(id: string): string {
	const item = readCollection().records.find(
		({ record }) => record.kind === 'need' && record.id === id,
	);
	if (!item || item.record.kind !== 'need') throw new Error(`Unknown need ${id}`);
	return `${item.page}#${encodeURIComponent(item.record.bodyRef)}`;
}
export function updateCallAddress(stackId: string, callId: string): void {
	const address = new URL(location.href);
	address.searchParams.delete('need');
	address.searchParams.set('stack', stackId);
	address.searchParams.set('call', callId);
	history.replaceState(history.state, '', address);
}
