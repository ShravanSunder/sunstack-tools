import { expect, it } from 'vitest';

import { type DeliverableView, deliverableViewSchema } from '../../core/deliverables-view.ts';
import {
	initiativeMilestones,
	initiativeWork,
	orderInitiativeProjects,
} from './initiative-order.ts';

function core(
	id: string,
	state: string,
	dependsOn: string[] = [],
	candidate?: string,
): DeliverableView {
	return deliverableViewSchema.parse({
		kind: 'core',
		id,
		title: id,
		dependsOn,
		state,
		reasons: [],
		progress: { state: 'planned', reason: 'Start the scoped implementation tasks.' },
		implementation: { tasks: [], evidence: [], ...(candidate ? { candidate } : {}) },
		verification: { checks: [] },
		links: {
			plan: { page: 'index.html', fragment: id },
			rules: [],
			designItems: [],
			governingItems: [],
			verificationChecks: [],
		},
	});
}

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

it('expands nested milestones and ranks what blocks the most open work', (): void => {
	// Arrange
	const deliverables = {
		'link/DEL-01': core('link/DEL-01', 'awaiting-verification', [], 'abc123'),
		'sessions/DEL-01': core('sessions/DEL-01', 'dependency-blocked', ['link/DEL-01']),
		'sessions/DEL-02': core('sessions/DEL-02', 'dependency-blocked', ['sessions/DEL-01']),
		'staging/DEL-01': core('staging/DEL-01', 'accepted'),
		'harness/DEL-01': core('harness/DEL-01', 'planned'),
	};

	// Act
	const milestones = initiativeMilestones(
		{
			registered: {
				title: 'Registered',
				check: 'A session registers.',
				includes: ['sessions/DEL-01'],
			},
			approval: {
				title: 'Approval',
				check: 'One approval is answered.',
				includes: ['registered', 'staging/DEL-01'],
			},
		},
		deliverables,
	);
	const work = initiativeWork(deliverables);

	// Assert
	expect(milestones['approval']).toMatchObject({
		deliverables: ['sessions/DEL-01', 'staging/DEL-01'],
		state: 'open',
	});
	expect(work.readyNow).toEqual(['harness/DEL-01']);
	expect(work.blocksMost[0]).toEqual({ id: 'link/DEL-01', waiting: 2 });
	expect(work.builtNotAccepted).toEqual(['link/DEL-01']);
	expect(() =>
		initiativeMilestones(
			{ loop: { title: 'Loop', check: 'Never.', includes: ['loop'] } },
			deliverables,
		),
	).toThrow(/include each other/);
});
