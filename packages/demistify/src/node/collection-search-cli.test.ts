import { execFile } from 'node:child_process';
import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { expect, it } from 'vitest';

const run = promisify(execFile);
const cli = fileURLToPath(new URL('../../bin/demistify.ts', import.meta.url));

it('indexes actual local files and leaves them unchanged when another input cannot be indexed', async (): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), 'demistify-search-cli-'));
	const first = join(directory, 'index.html');
	const second = join(directory, 'proof.html');
	const firstSource = '<title>Queue</title><h1>Queue</h1><h2>Receipt</h2>';
	await Promise.all([
		writeFile(first, firstSource),
		writeFile(second, '<title>Proof</title><ex-markdown id="proof" src="proof.md"></ex-markdown>'),
	]);
	await expect(run(process.execPath, [cli, 'index', first, second])).rejects.toThrow();
	expect(await readFile(first, 'utf8')).toBe(firstSource);
	await writeFile(
		join(directory, 'proof.md'),
		'## Durable receipt\n\nObserve the stored identity.',
	);
	const result = await run(process.execPath, [cli, 'index', first, second]);
	expect(result.stdout).toContain('Indexed 2 pages and 4 destinations.');
	const indexed = await readFile(first, 'utf8');
	expect(indexed).toContain('proof.html#proof--durable-receipt');
	expect(indexed).toContain('id="heading-receipt"');
	await run(process.execPath, [cli, 'index', first, second]);
	expect(await readFile(first, 'utf8')).toBe(indexed);
});

it('indexes embedded Markdown headings and replaces them after source regeneration', async (): Promise<void> => {
	// Arrange: the reader has prepared HTML, without a sibling Markdown fetch.
	const directory = await mkdtemp(join(tmpdir(), 'demistify-embedded-search-'));
	const first = join(directory, 'index.html');
	const live = join(directory, 'proof.source.html');
	const delivered = join(directory, 'proof.html');
	const markdown = join(directory, 'proof.md');
	await Promise.all([
		writeFile(first, '<title>Overview</title><h1>Overview</h1>'),
		writeFile(
			live,
			'<title>Proof</title><h1>Proof</h1><ex-markdown id="proof" src="proof.md"></ex-markdown>',
		),
		writeFile(markdown, '## Before cleanup\n\nLiteral </script> and <tag>.\n\n### Read receipt\n'),
	]);

	// Act: use the real authoring CLI in the documented order.
	await run(process.execPath, [cli, 'embed', live, delivered]);
	const initial = await run(process.execPath, [cli, 'index', first, delivered]);

	// Assert: other pages can search both actual Markdown destinations.
	expect(initial.stdout).toContain('Indexed 2 pages and 4 destinations.');
	const indexed = await readFile(first, 'utf8');
	expect(indexed).toContain('proof.html#proof--before-cleanup');
	expect(indexed).toContain('proof.html#proof--read-receipt');

	// Act: regenerate after the source heading changes.
	await writeFile(
		markdown,
		'## After cleanup\n\nLiteral </script> and <tag>.\n\n### Read receipt\n',
	);
	await run(process.execPath, [cli, 'embed', live, delivered]);
	await run(process.execPath, [cli, 'index', first, delivered]);

	// Assert: the new title replaces the old destination rather than accumulating it.
	const updated = await readFile(first, 'utf8');
	expect(updated).toContain('proof.html#proof--after-cleanup');
	expect(updated).not.toContain('proof--before-cleanup');
	expect(updated.match(/data-ex-search-index/g)).toHaveLength(1);
});
