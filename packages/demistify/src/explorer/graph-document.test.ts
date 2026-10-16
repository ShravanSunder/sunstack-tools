import { describe, expect, it } from 'vitest';

import { checkHtml } from '../node/check.ts';
import {
	compositionView,
	parseGraphDocument,
	tracedEdges,
	type GraphDocument,
} from './graph-document.ts';

const input = {
	version: 1,
	sourceBase: 'https://example.com/source/',
	title: 'Session ownership',
	description: '',
	note: '',
	groups: [{ id: 'services', label: 'Services', kind: 'method' }],
	nodes: ['client', 'registry', 'session'].map((id) => ({
		id,
		label: id,
		description: '',
		group: 'services',
		issue: '',
		status: 'current',
		sources: [],
	})),
	edges: [
		{
			id: 'lookup',
			source: 'client',
			target: 'registry',
			expansion: 'composition',
			status: 'current',
			label: 'lookup',
			why: '',
			when: '',
			use: '',
			sources: [],
		},
		{
			id: 'store',
			source: 'registry',
			target: 'session',
			expansion: 'composition',
			status: 'current',
			label: 'store',
			why: '',
			when: '',
			use: '',
			sources: [],
		},
		{
			id: 'direct',
			source: 'client',
			target: 'session',
			expansion: 'composition',
			status: 'proposed',
			label: 'direct',
			why: '',
			when: '',
			use: '',
			sources: [],
		},
		{
			id: 'peer',
			source: 'session',
			target: 'client',
			expansion: 'related',
			status: 'current',
			label: 'peer',
			why: '',
			when: '',
			use: '',
			sources: [],
		},
	],
	tables: [],
} satisfies GraphDocument;

describe('explorer content and composition contract', (): void => {
	it('distinguishes proposal edges, preserves shared targets and traces only the focused route', (): void => {
		const document = parseGraphDocument(input);
		const current = compositionView(document, 'client', 'current');
		const proposed = compositionView(document, 'client', 'proposed');
		expect(current.edges.map((edge): string => edge.id)).toEqual(['lookup', 'store']);
		expect(proposed.nodes.map((node): string => node.id)).toEqual([
			'client',
			'registry',
			'session',
		]);
		expect(new Set(proposed.edges.map((edge): string => edge.id))).toEqual(
			new Set(['lookup', 'store', 'direct']),
		);
		expect(tracedEdges(proposed, 'registry')).toEqual(new Set(['lookup', 'store']));
	});
	it('terminates composition cycles without losing their edge identities', (): void => {
		const document = parseGraphDocument({
			...input,
			edges: [
				...input.edges,
				{ ...input.edges[0], id: 'cycle', source: 'session', target: 'client' },
			],
		});
		const view = compositionView(document, 'client', 'current');
		expect(view.nodes).toHaveLength(3);
		expect(view.edges.map((edge): string => edge.id)).toContain('cycle');
	});
	it('rejects ambiguous identities, dangling references and malformed table cells before rendering', (): void => {
		expect((): GraphDocument =>
			parseGraphDocument({ ...input, nodes: [...input.nodes, input.nodes[0]] }),
		).toThrow('duplicate id');
		expect((): GraphDocument =>
			parseGraphDocument({
				...input,
				nodes: [...input.nodes, { ...input.nodes[0], id: '__issues' }],
			}),
		).toThrow('Reserved node id');
		expect((): GraphDocument =>
			parseGraphDocument({
				...input,
				tables: [
					{
						id: 'connections',
						title: '',
						description: '',
						placement: 'details',
						filter: 'all',
						columns: [],
						rows: [],
					},
				],
			}),
		).toThrow('Reserved detail table id');
		expect((): GraphDocument =>
			parseGraphDocument({ ...input, edges: [{ ...input.edges[0], target: 'not-a-node' }] }),
		).toThrow('Unknown graph node');
		expect((): GraphDocument =>
			parseGraphDocument({
				...input,
				tables: [
					{
						id: 'facts',
						title: 'Facts',
						description: '',
						placement: 'details',
						filter: 'all',
						columns: ['Fact'],
						rows: [{ id: 'r1', nodes: [], cells: [] }],
					},
				],
			}),
		).toThrow('Wrong cell count');
	});
});

const readSource = async (path: string): Promise<string | undefined> =>
	path === 'graph.json' ? JSON.stringify(input) : undefined;

it('checks the actual referenced graph through the HTML validator, including invalid selection and missing files', async (): Promise<void> => {
	const html =
		'<!doctype html><html><head><link rel="stylesheet" href="demistify/theme.css"><script type="module" src="demistify/runtime.js"></script></head><body><ex-explorer src="graph.json" selected="client"></ex-explorer></body></html>';
	expect(await checkHtml(html, { readSource })).toEqual([]);
	const invalidSelection = await checkHtml(
		html.replace('selected="client"', 'selected="label-not-id"'),
		{ readSource },
	);
	expect(invalidSelection.map((finding): string => finding.rule)).toEqual(['explorer']);
	expect(invalidSelection[0]?.message).toContain('Unknown initial graph selection');
	const missing = await checkHtml(html.replace('graph.json', 'missing.json'), { readSource });
	expect(missing.map((finding): string => finding.rule)).toEqual(['explorer']);
	expect(missing[0]?.message).toContain('Cannot read explorer source');
});
