import { readFile } from 'node:fs/promises';

import { expect, it } from 'vitest';

import { prepareIslandContent } from './islands-packager.ts';

it('packages real Markdown tables and code without losing source whitespace', (): void => {
	const source =
		'<ex-markdown><script type="text/markdown">\n\t# Design\n\n\t| Owner | Job |\n\t| --- | --- |\n\t| Lead | Acceptance |\n\n\t```ts\n\tconst ready = true;\n\t```\n</script></ex-markdown>';
	const output = prepareIslandContent(source, {}, true);
	expect(output).toContain('<h1>Design</h1>');
	expect(output).toContain('<table>');
	expect(output).toContain('<td>Acceptance</td>');
	expect(output).toContain('<code class="language-ts">const ready = true;');
	expect(output).toContain('prepared');
});

it('resolves SVG references and refuses an absent source', (): void => {
	const source = '<ex-mermaid src="diagram.mmd"></ex-mermaid>';
	expect(
		prepareIslandContent(source, { 'diagram.mmd': 'flowchart TD\n  Planning[Planning]' }, false),
	).toContain('<svg');
	expect((): string => prepareIslandContent(source, {}, false)).toThrow(
		'Unknown Mermaid island source',
	);
});

it('renders Mermaid fences as islands while keeping other code fences literal', (): void => {
	const source =
		'<ex-markdown><script type="text/markdown">\n# Proposal\n\n```mermaid\nflowchart TD\n  Client --> Session\n```\n\n```text\nflowchart TD\n  Client --> Session\n```\n</script></ex-markdown>';
	const output = prepareIslandContent(source, {}, true);
	expect(output).toMatch(/<ex-mermaid[^>]*prepared/);
	expect(output).toContain('<svg');
	expect(output).not.toContain('class="language-mermaid"');
	expect(output).toContain('<code class="language-text">flowchart TD');
});

it('composes fenced and explicit Mermaid islands together inside one comparison', (): void => {
	const source =
		'<ex-compare><section slot="before"><ex-markdown><script type="text/markdown">\n## Current\n\n```mermaid\nflowchart TD\n  Client --> Socket --> Session\n```\n</script></ex-markdown></section><section slot="after"><ex-mermaid><script type="text/plain">flowchart TD\n  Client --> Registry --> Session\n</script></ex-mermaid></section></ex-compare>';
	const output = prepareIslandContent(source, {}, true);
	expect(output.match(/<ex-mermaid[^>]*prepared/g)).toHaveLength(2);
	expect(output).toContain('<h2>Current</h2>');
	expect(output).toContain('<ex-compare>');
});

it('renders the real formatted showcase with its Markdown table and all three diagrams', async (): Promise<void> => {
	const source = await readFile('experiments/html-islands/composition-demo.html', 'utf8');
	const output = prepareIslandContent(source, {}, true);
	expect(output.match(/<ex-mermaid[^>]*prepared/g)).toHaveLength(3);
	expect(output).toContain('<table>');
	expect(output).toContain('<td>Registry</td>');
	expect(output).toContain('<code class="language-text">flowchart TD');
});

it('exports the actual standalone page with its lazy runtime dependency catalog', async (): Promise<void> => {
	const artifact = await readFile('public/experiments/html-islands-rendered.html', 'utf8');
	const catalogText =
		/<script id="demistify-offline-modules" type="application\/json">([\s\S]*?)<\/script>/.exec(
			artifact,
		)?.[1];
	expect(catalogText).toBeDefined();
	const catalog: unknown = JSON.parse(catalogText ?? 'null');
	expect(catalog).toEqual(
		expect.arrayContaining([
			expect.objectContaining({ name: 'runtime.js' }),
			expect.objectContaining({ name: 'diagram-worker.js' }),
		]),
	);
	expect(artifact).not.toContain('src="islands/runtime.js"');
});
