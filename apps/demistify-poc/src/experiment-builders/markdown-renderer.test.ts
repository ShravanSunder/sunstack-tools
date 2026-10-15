import { expect, it } from 'vitest';

import { renderMarkdownDirectives } from './markdown-renderer.ts';

it('resolves referenced diagrams and retains prose and decision meaning', async (): Promise<void> => {
	const renderedHtml = await renderMarkdownDirectives(
		'# A design\n\n::diagram{src="fixture.d2"}\n\n:::decision\nThe store owns lifetime.\n:::',
		{ 'fixture.d2': '<svg><text>Store</text></svg>' },
	);
	expect(renderedHtml).toContain('<h1>A design</h1>');
	expect(renderedHtml).toContain('<svg><text>Store</text></svg>');
	expect(renderedHtml).toContain('<aside class="decision">');
	expect(renderedHtml).toContain('The store owns lifetime.');
});

it('refuses missing references and unsupported directives', async (): Promise<void> => {
	await expect(renderMarkdownDirectives('::diagram{src="missing.d2"}', {})).rejects.toThrow(
		'Unknown diagram reference',
	);
	await expect(renderMarkdownDirectives(':::unknown\nContent\n:::', {})).rejects.toThrow(
		'Unsupported directive',
	);
});
