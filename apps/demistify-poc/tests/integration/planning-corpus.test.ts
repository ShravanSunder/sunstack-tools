import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';

import { normalizeIslandText } from '@sunstack/demistify/source-text';
import { expect, it } from 'vitest';
import { z } from 'zod';

import {
	applyGraphChanges,
	changeSetSchema,
	graphSchema,
} from '../../src/experiment-builders/graph-changes.ts';
import {
	planningViewsSchema,
	planningMermaid,
} from '../../src/experiment-builders/planning-fixture.ts';

type JsonContent = z.infer<ReturnType<typeof z.json>>;

function collectJsonText(value: JsonContent): string {
	if (typeof value === 'string') return value;
	if (Array.isArray(value)) return value.map(collectJsonText).join('\n');
	if (value !== null && typeof value === 'object')
		return Object.values(value).map(collectJsonText).join('\n');
	return '';
}

it('keeps the complete explanation and change story in every counted author input', async (): Promise<void> => {
	const views = planningViewsSchema.parse(
		JSON.parse(await readFile('fixtures/planning-views.json', 'utf8')),
	);
	const authorPaths = [
		'experiments/visual-plan/planning.mdx',
		'experiments/quick-json/planning.json',
		'experiments/markdown-directives/planning.md',
		'experiments/graph-diff/planning.md',
		'experiments/html-islands/planning.html',
	];
	const authorSources = await Promise.all(
		authorPaths.map(async (authorPath) => ({
			authorPath,
			source: await readFile(authorPath, 'utf8'),
		})),
	);
	for (const { authorPath, source } of authorSources) {
		const authorText = (
			authorPath.endsWith('.json') ? collectJsonText(z.json().parse(JSON.parse(source))) : source
		).replace(/\s+/g, ' ');
		for (const view of Object.values(views)) {
			expect(authorText, authorPath).toContain(view.title);
			for (const node of view.nodes) {
				for (const fact of [node.label, node.subtitle, node.purpose, node.boundary, node.source])
					expect(authorText, `${authorPath}: ${node.id}`).toContain(fact.replace(/\s+/g, ' '));
			}
		}
		expect(authorText, authorPath).toContain('Implemented source and the planned extension');
		expect(authorText, authorPath).toContain('six planned nodes and six dependency relationships');
	}
});

it('preserves the exact diagram data extracted from the supplied original', async (): Promise<void> => {
	const originalHtml = await readFile('references/planning-dag-original.html', 'utf8');
	const originalJson = /const graphData = ([\s\S]*?);\nconst canvas/.exec(originalHtml)?.[1];
	if (!originalJson) throw new Error('The supplied original has no graph data.');
	const storedViews = planningViewsSchema.parse(
		JSON.parse(await readFile('fixtures/planning-views.json', 'utf8')),
	);
	expect(storedViews).toEqual(JSON.parse(originalJson));
	expect(
		Object.values(storedViews).reduce((total, view): number => total + view.nodes.length, 0),
	).toBe(25);
	expect(
		Object.values(storedViews).reduce((total, view): number => total + view.edges.length, 0),
	).toBe(24);
	expect(planningMermaid(storedViews.roles)).toContain('implementer <-.->|facts| verifier');
});

it('retains every job and boundary in the protected HTML Islands author input', async (): Promise<void> => {
	const storedViews = planningViewsSchema.parse(
		JSON.parse(await readFile('fixtures/planning-views.json', 'utf8')),
	);
	const islandHtml = await readFile('experiments/html-islands/planning.html', 'utf8');
	const authorText = Array.from(
		islandHtml.matchAll(/<script type="text\/markdown">([\s\S]*?)<\/script>/g),
		(match): string => normalizeIslandText(match[1] ?? ''),
	).join('\n');
	for (const view of Object.values(storedViews)) {
		for (const node of view.nodes) {
			expect(authorText).toContain(node.purpose);
			expect(authorText).toContain(node.boundary);
		}
	}
});

it('derives the same roadmap from the supplied base and the semantic patch', async (): Promise<void> => {
	const sources = await Promise.all([
		readFile('fixtures/planning-base.json', 'utf8'),
		readFile('experiments/graph-diff/planning-changes.json', 'utf8'),
		readFile('fixtures/planning-views.json', 'utf8'),
	]);
	const proposedGraph = applyGraphChanges(
		graphSchema.parse(JSON.parse(sources[0] ?? '')),
		changeSetSchema.parse(JSON.parse(sources[1] ?? '')),
	);
	const views = planningViewsSchema.parse(JSON.parse(sources[2] ?? ''));
	expect(proposedGraph.nodes).toEqual(
		views.roadmap.nodes.map((node) => ({ id: node.id, label: node.label })),
	);
	expect(proposedGraph.edges.map((edge) => [edge.from, edge.to])).toEqual(views.roadmap.edges);
});

it('reports the exact complete usage files and keeps library assets outside the manifest', async (): Promise<void> => {
	const measurements = z
		.array(
			z.object({
				id: z.string(),
				usageBytes: z.number().positive(),
				usageTokens: z.number().positive(),
				tokenEncoding: z.literal('o200k_base'),
				authorFiles: z.array(
					z.object({ path: z.string(), bytes: z.number(), tokens: z.number(), sha256: z.string() }),
				),
			}),
		)
		.parse(JSON.parse(await readFile('public/experiments/measurements.json', 'utf8')));
	expect(measurements).toHaveLength(5);
	const fileChecks = measurements.flatMap((measurement) => {
		expect(measurement.usageBytes).toBe(
			measurement.authorFiles.reduce((total, file): number => total + file.bytes, 0),
		);
		expect(measurement.usageTokens).toBe(
			measurement.authorFiles.reduce((total, file): number => total + file.tokens, 0),
		);
		expect(measurement.authorFiles).toHaveLength(measurement.id === 'graph-diff' ? 2 : 1);
		return measurement.authorFiles.map(async (file): Promise<void> => {
			expect(file.path).toMatch(new RegExp(`^experiments/${measurement.id}/`));
			expect(file.path).not.toMatch(/upstream|capabilities|runtime|theme/);
			const source = await readFile(file.path, 'utf8');
			expect(file.bytes).toBe(Buffer.byteLength(source));
			expect(file.sha256).toBe(createHash('sha256').update(source).digest('hex'));
		});
	});
	await Promise.all(fileChecks);
	const islandSource = await readFile('experiments/html-islands/planning.html', 'utf8');
	expect(islandSource).not.toContain('<style');
	expect(islandSource).not.toMatch(/<ex-(?:layer|timeline|file-tree)/);
	expect(islandSource).not.toMatch(
		/<script[^>]*type="module"[^>]*>(?!<\/script)[\s\S]+?<\/script>/,
	);
});
