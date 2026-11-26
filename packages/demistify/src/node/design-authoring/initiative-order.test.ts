import { expect, it } from 'vitest';

import { type DeliverableView, deliverableViewSchema } from '../../core/deliverables-view.ts';
import { initiativeWork, projectProgress } from './initiative-order.ts';

function core(
	id: string,
	state: string,
	blockedBy: string[] = [],
	candidate?: string,
	steps: string[] = [],
): DeliverableView {
	return deliverableViewSchema.parse({
		kind: 'core',
		id,
		title: id,
		blockedBy,
		state,
		reasons: [],
		progress: { state: 'planned', reason: 'Start the scoped issues.' },
		implementation: {
			issues: steps.map((step) => ({ id: step, title: step, state: 'open' })),
			evidence: [],
			...(candidate ? { candidate } : {}),
		},
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

it('ranks what blocks the most open work across projects and lists ready and built work', (): void => {
	// Arrange
	const deliverables = {
		'link/DEL-01': core('link/DEL-01', 'awaiting-verification', [], 'abc123'),
		'link/DEL-02': core('link/DEL-02', 'planned'),
		'sessions/DEL-01': core('sessions/DEL-01', 'dependency-blocked', ['link/DEL-01']),
		'sessions/DEL-02': core('sessions/DEL-02', 'dependency-blocked', ['sessions/DEL-01']),
		'staging/DEL-01': core('staging/DEL-01', 'accepted'),
		'harness/DEL-01': core('harness/DEL-01', 'planned', [], undefined, ['step-one']),
		'staging/DEL-02': core('staging/DEL-02', 'planned', [], undefined, ['step-one']),
	};
	const numbers: Record<string, number> = { harness: 2, staging: 1, link: 1, sessions: 2 };

	// Act
	const work = initiativeWork(deliverables, (id) => numbers[id.slice(0, id.indexOf('/'))] ?? 9);

	// Assert
	expect(work.readyNow).toEqual(['staging/DEL-02', 'harness/DEL-01']);
	expect(work.needsPlan).toEqual([
		'link/DEL-01',
		'link/DEL-02',
		'sessions/DEL-01',
		'sessions/DEL-02',
	]);
	expect(work.blocksMost[0]).toEqual({ id: 'link/DEL-01', waiting: 2 });
	expect(work.builtNotAccepted).toEqual(['link/DEL-01']);
});

it('derives a project state from its own deliverables only', (): void => {
	// Arrange
	const blocked = [core('sessions/DEL-01', 'dependency-blocked', ['link/DEL-01'])];
	const mixed = [
		core('sessions/DEL-01', 'dependency-blocked', ['link/DEL-01']),
		core('sessions/DEL-02', 'implementation-in-progress'),
	];

	// Act
	const blockedState = projectProgress(blocked);
	const mixedState = projectProgress(mixed);

	// Assert
	expect(blockedState).toEqual({
		state: 'dependency-blocked',
		reason: 'Every open core deliverable waits on another: link/DEL-01.',
	});
	expect(mixedState.state).toBe('in-progress');
	expect(
		projectProgress([
			core('sessions/DEL-01', 'dependency-blocked', ['link/DEL-01']),
			core('sessions/DEL-02', 'design-blocked'),
		]),
	).toEqual({
		state: 'design-blocked',
		reason: 'sessions/DEL-02 wait on an open owner decision; the rest wait on link/DEL-01.',
	});
});
