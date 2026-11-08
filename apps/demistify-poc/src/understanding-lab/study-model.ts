export interface OwnerExample {
	readonly id: string;
	readonly name: string;
	readonly category: string;
	readonly role: 'boundary' | 'store' | 'state';
	readonly icon: string;
	readonly description: string;
}
export const owners: readonly OwnerExample[] = [
	{
		id: 'gateway',
		name: 'Gateway',
		category: 'Boundary',
		role: 'boundary',
		icon: 'globe',
		description: 'Routes reconnect requests.',
	},
	{
		id: 'registry',
		name: 'Registry',
		category: 'Store',
		role: 'store',
		icon: 'database',
		description: 'Owns session lookup.',
	},
	{
		id: 'session',
		name: 'Session',
		category: 'State',
		role: 'state',
		icon: 'cpu',
		description: 'Retains running state.',
	},
];
