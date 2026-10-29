import { execFile } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { expect, it } from 'vitest';

const run = promisify(execFile);
const archive = fileURLToPath(new URL('../../dist/demistify-cli.tgz', import.meta.url));
const pageHead =
	'<head><title>Guide</title><link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css"><script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script></head>';

it('runs the packed authoring commands outside a checkout without browser dependencies', async (): Promise<void> => {
	// Arrange: only the released tarball and authored sources exist in the consumer directory.
	const directory = await mkdtemp(join(tmpdir(), 'demistify-cli-consumer-'));
	try {
		await run('tar', ['-xzf', archive, '-C', directory]);
		const cli = join(directory, 'package/demistify.js');
		const source = join(directory, 'guide.source.html');
		const reader = join(directory, 'guide.html');
		await writeFile(join(directory, 'guide.md'), '## Ownership\n\nThe Registry owns the record.');
		await writeFile(
			source,
			`${pageHead}<ex-document><ex-markdown id="guide" src="guide.md"></ex-markdown></ex-document>`,
		);

		// Act: prepare and lint through the bundled executable, including its lazy diagram engine.
		await run(process.execPath, [cli, 'embed', source, reader], { cwd: directory });
		await run(process.execPath, [cli, 'index', reader], { cwd: directory });
		const checked = await run(process.execPath, [cli, 'check', reader], { cwd: directory });
		expect(checked.stdout).toContain('1 file: 0 errors');
		expect(await readFile(reader, 'utf8')).toContain('The Registry owns the record.');
		await writeFile(
			reader,
			`${pageHead}<ex-document><ex-mermaid><script type="text/plain">flowchart TD\n A[Request] --> B[Accepted]</script></ex-mermaid></ex-document>`,
		);
		expect(
			(await run(process.execPath, [cli, 'check', reader], { cwd: directory })).stdout,
		).toContain('1 file: 0 errors');

		// Assert: malformed authoring and an omitted input cannot silently pass.
		await writeFile(reader, `${pageHead}<ex-document><ex-unknown></ex-unknown></ex-document>`);
		await expect(
			run(process.execPath, [cli, 'check', reader], { cwd: directory }),
		).rejects.toMatchObject({ code: 1 });
		await expect(run(process.execPath, [cli, 'check'], { cwd: directory })).rejects.toMatchObject({
			code: 1,
		});
	} finally {
		await rm(directory, { recursive: true, force: true });
	}
});
