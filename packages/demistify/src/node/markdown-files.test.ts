import { expect, it } from 'vitest';

import { checkHtml } from './check.ts';
import { prepareIslandContent } from './packaging.ts';

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
