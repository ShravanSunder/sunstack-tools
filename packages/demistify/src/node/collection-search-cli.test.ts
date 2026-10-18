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
