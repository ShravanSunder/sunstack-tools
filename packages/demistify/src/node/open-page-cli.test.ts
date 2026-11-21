import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdir, mkdtemp, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';

import { expect, it } from 'vitest';

const cli = fileURLToPath(new URL('../../bin/demistify.ts', import.meta.url));

it('opens authored pages and their local includes within the selected site root', async (): Promise<void> => {
	const scratch = await mkdtemp(join(tmpdir(), 'demistify-open-'));
	const root = join(scratch, 'site');
	const pages = join(root, 'project');
	const page = join(pages, 'review notes.html');
	await mkdir(pages, { recursive: true });
	const html = '<ex-markdown src="../notes.md"></ex-markdown>';
	await Promise.all([
		writeFile(page, html),
		writeFile(join(root, 'notes.md'), '## Reviewed passage'),
		writeFile(join(root, 'project.generated.json'), '{"version":1}'),
		writeFile(join(root, '.hidden.json'), '{}'),
		writeFile(join(scratch, 'outside.md'), 'Outside'),
	]);
	await symlink(join(scratch, 'outside.md'), join(root, 'outside.md'), 'file');
	const processHandle = spawn(
		process.execPath,
		[cli, 'open', page, '--site-root', root, '--no-browser'],
		{ stdio: ['ignore', 'pipe', 'inherit'] },
	);
	const stopped = once(processHandle, 'close');
	const lines = createInterface({ input: processHandle.stdout });
	try {
		const output: unknown = await once(lines, 'line', { signal: AbortSignal.timeout(5000) });
		if (!Array.isArray(output) || typeof output[0] !== 'string')
			throw new Error('Expected the preview address on stdout.');
		const line = output[0];
		expect(line).toMatch(/^Open http:\/\/127\.0\.0\.1:\d+\/project\/review%20notes\.html$/);
		const url = new URL(line.slice('Open '.length));
		const [document, markdown, data, outside, hidden] = await Promise.all([
			fetch(url),
			fetch(new URL('../notes.md', url)),
			fetch(new URL('../project.generated.json', url)),
			fetch(new URL('../outside.md', url)),
			fetch(new URL('../.hidden.json', url)),
		]);
		expect(await document.text()).toBe(html);
		expect(await markdown.text()).toBe('## Reviewed passage');
		expect(data.headers.get('content-type')).toBe('application/json');
		expect(await data.text()).toBe('{"version":1}');
		expect(outside.status).toBe(404);
		expect(hidden.status).toBe(404);
	} finally {
		lines.close();
		processHandle.kill();
		await stopped;
		await rm(scratch, { recursive: true, force: true });
	}
}, 10_000);
