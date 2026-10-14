import { readFile } from 'node:fs/promises';

import { expect, it } from 'vitest';

import { renderD2Sources } from '../../src/experiment-builders/d2-renderer.ts';
import {
	changeSetSchema,
	graphSchema,
	graphToD2,
	overlayGraph,
} from '../../src/experiment-builders/graph-changes.ts';
import { applySvgTheme } from '../../src/experiment-builders/svg-theme.ts';

it('uses the shared semantic theme for additions, removals and modifications in real D2 output', async (): Promise<void> => {
	const sources = await Promise.all([
		readFile('fixtures/session-graph.json', 'utf8'),
		readFile('experiments/graph-diff/changes.json', 'utf8'),
		readFile('../../packages/demistify/src/theme.css', 'utf8'),
	]);
	const graph = graphSchema.parse(JSON.parse(sources[0] ?? ''));
	const changes = changeSetSchema.parse(JSON.parse(sources[1] ?? ''));
	const darkTheme = /:root\[data-theme='dark'\]\s*\{([\s\S]*?)\}/.exec(sources[2] ?? '')?.[1] ?? '';
	const palette = Object.fromEntries(
		Array.from(
			darkTheme.matchAll(/--([\w-]+):\s*(#[\da-f]{6});/gi),
			(match): readonly [string, string] => [match[1] ?? '', match[2] ?? ''],
		).filter(([name]) => name === 'ink' || name === 'sheet' || name?.startsWith('diff-')),
	);
	const rendered = await renderD2Sources({
		changes: graphToD2(overlayGraph(graph, changes), palette, changes),
	});
	const themedSvg = applySvgTheme(rendered['changes'] ?? '', palette);
	expect(themedSvg).toContain('var(--diff-added-bg,');
	expect(themedSvg).toContain('var(--diff-removed-bg,');
	expect(themedSvg).toContain('var(--diff-modified-bg,');
	expect(themedSvg).toContain('var(--sheet,');
	expect(themedSvg).not.toContain('var(--codex');
});
