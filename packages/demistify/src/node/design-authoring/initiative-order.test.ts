import { expect, it } from 'vitest';

import { orderInitiativeProjects } from './initiative-order.ts';

it('orders projects after the projects they depend on, keeping authored order for ties', (): void => {
	// Arrange
	const projects = new Map<string, readonly string[]>([
		['first-message', ['staging', 'messages']],
		['messages', []],
		['staging', []],
		['approvals', ['messages']],
	]);

	// Act
	const { order, loops } = orderInitiativeProjects(projects);

	// Assert
	expect(order).toEqual(['messages', 'staging', 'approvals', 'first-message']);
	expect(loops).toEqual([]);
});

it('reports projects that depend on each other as one loop and still lists every project', (): void => {
	// Arrange
	const projects = new Map<string, readonly string[]>([
		['sign-in', ['central-path']],
		['central-path', ['sign-in']],
		['staging', []],
		['first-message', ['sign-in']],
	]);

	// Act
	const { order, loops } = orderInitiativeProjects(projects);

	// Assert
	expect(order).toEqual(['staging', 'sign-in', 'central-path', 'first-message']);
	expect(loops).toEqual([['sign-in', 'central-path']]);
});
