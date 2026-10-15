import { readFile } from 'node:fs/promises';

import { describe, expect, it } from 'vitest';

import { applyGraphChanges, graphSchema, changeSetSchema } from './graph-changes.ts';

const baseGraph = graphSchema.parse(
	JSON.parse(await readFile('fixtures/session-graph.json', 'utf8')),
);
const changeSet = changeSetSchema.parse(
	JSON.parse(await readFile('experiments/graph-diff/changes.json', 'utf8')),
);

describe('semantic architecture changes', (): void => {
	it('does not rewrite the source patch when an added node is subsequently renamed', (): void => {
		const sourcePatch = changeSetSchema.parse({
			base: 'fixture',
			changes: [
				{ kind: 'add-node', node: { id: 'worker', label: 'Original worker' } },
				{ kind: 'change-node', id: 'worker', label: 'Renamed worker' },
			],
		});
		const originalPatch = structuredClone(sourcePatch);
		const derivedGraph = applyGraphChanges(baseGraph, sourcePatch);
		expect(derivedGraph.nodes.find((node): boolean => node.id === 'worker')?.label).toBe(
			'Renamed worker',
		);
		expect(sourcePatch).toEqual(originalPatch);
	});
	it('derives the proposed graph without changing the base', (): void => {
		const proposedGraph = applyGraphChanges(baseGraph, changeSet);
		expect(proposedGraph.nodes.map((node): string => node.id)).toEqual([
			'client',
			'gateway',
			'session_store',
			'reconnect_worker',
		]);
		expect(proposedGraph.edges.map((edge): string => edge.id)).toEqual([
			'client_gateway',
			'gateway_store',
			'store_worker',
			'worker_gateway',
		]);
		expect(proposedGraph.nodes.find((node): boolean => node.id === 'session_store')?.label).toBe(
			'Session store (owns lifetime)',
		);
		expect(baseGraph.nodes.map((node): string => node.id)).toContain('socket_cache');
		expect(baseGraph.edges).toHaveLength(3);
	});

	it('rejects removing a node with dangling edges', (): void => {
		expect((): unknown =>
			applyGraphChanges(baseGraph, {
				base: 'fixture',
				changes: [{ kind: 'remove-node', id: 'socket_cache' }],
			}),
		).toThrow('Unknown edge endpoint');
	});

	it('rejects unknown and duplicate identities', (): void => {
		expect((): unknown =>
			applyGraphChanges(baseGraph, {
				base: 'fixture',
				changes: [{ kind: 'change-node', id: 'missing', label: 'Changed' }],
			}),
		).toThrow('Unknown node');
		expect((): unknown =>
			applyGraphChanges(baseGraph, {
				base: 'fixture',
				changes: [{ kind: 'add-node', node: { id: 'client', label: 'Second client' } }],
			}),
		).toThrow('Duplicate node');
	});

	it('propagates a canonical label edit to the derived graph', (): void => {
		const editedGraph = structuredClone(baseGraph);
		const editedClient = editedGraph.nodes.find((node): boolean => node.id === 'client');
		if (!editedClient) throw new Error('The canonical fixture is missing its client.');
		editedClient.label = 'Desktop client';
		expect(applyGraphChanges(editedGraph, changeSet).nodes[0]?.label).toBe('Desktop client');
	});
});
