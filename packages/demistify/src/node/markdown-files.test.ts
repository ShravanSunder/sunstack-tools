import { expect, it } from 'vitest';

import { checkHtml } from './check.ts';
import { embedMarkdownFiles } from './markdown-files.ts';
import { prepareIslandContent } from './packaging.ts';

it('keeps shared Cloudflare assets and image paths external when embedding Markdown', (): void => {
	const output = embedMarkdownFiles(
		'<!doctype html><link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css"><script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script><ex-document><ex-markdown src="notes.md"></ex-markdown><img src="captures/mobile.png" alt="Mobile"></ex-document>',
		{ 'notes.md': '# Notes\n\nExact authored source.' },
	);
	expect(output).toContain('href="https://demistify.getagentstudio.dev/v1/theme.css"');
	expect(output).toContain('src="https://demistify.getagentstudio.dev/v1/runtime.js"');
	expect(output).toContain('src="captures/mobile.png"');
	expect(output).not.toContain('src="notes.md"');
	expect(output).not.toContain('data:font');
	expect(output).not.toContain('data:image');
	expect(output).not.toContain('<style');
	expect(output.match(/<script\b/g)).toHaveLength(2);
	expect(output).toContain('Exact authored source.');
});

it('accepts a readable Markdown file and rejects a missing file at the author boundary', async (): Promise<void> => {
	const page =
		'<link href="theme.css"><script src="runtime.js"></script><ex-markdown src="notes.md"></ex-markdown>';
	const valid = await checkHtml(page, {
		readSource: async (path: string): Promise<string | undefined> =>
			path === 'notes.md' ? '# Notes\n\nSource prose.' : undefined,
	});
	expect(valid).toEqual([]);
	const missing = await checkHtml(page, { readSource: async (): Promise<undefined> => undefined });
	expect(missing).toEqual(
		expect.arrayContaining([
			expect.objectContaining({ severity: 'error', rule: 'markdown-source' }),
		]),
	);
});

it('embeds exact Markdown source safely so the consumer needs no sibling file fetch', (): void => {
	const source =
		'# Notes\n\nLiteral </script> and <tag>.\n\n```mermaid\nflowchart TD\n A --> B\n```\n';
	const output = prepareIslandContent(
		'<ex-markdown id="notes" src="notes.md"></ex-markdown>',
		{ 'notes.md': source },
		false,
	);
	expect(output).not.toContain('src="notes.md"');
	const metadata =
		/<script type="application\/json" data-ex-markdown-source(?:="")?>([\s\S]*?)<\/script>/.exec(
			output,
		)?.[1];
	expect(JSON.parse(metadata ?? 'null')).toEqual({ text: source, reference: 'notes.md' });
	expect(metadata).not.toContain('</script>');
	const prepared = prepareIslandContent(
		'<ex-markdown id="notes" src="notes.md"></ex-markdown>',
		{ 'notes.md': source },
		true,
	);
	expect(prepared).toContain('<h1>Notes</h1>');
	expect(prepared).toContain('<svg');
	const preparedMetadata =
		/<script type="application\/json" data-ex-markdown-source(?:="")?>([\s\S]*?)<\/script>/.exec(
			prepared,
		)?.[1];
	expect(JSON.parse(preparedMetadata ?? 'null')).toEqual({ text: source, reference: 'notes.md' });
	expect((): string =>
		prepareIslandContent('<ex-markdown src="missing.md"></ex-markdown>', {}, false),
	).toThrow('Unknown Markdown island source');
});
