import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { expect, test } from 'vitest';

const repositoryRoot = fileURLToPath(new URL('../../../../', import.meta.url));

test.each([
	{ event: 'pull_request', ref: 'refs/pull/1/merge', deploy: 'false' },
	{ event: 'workflow_dispatch', ref: 'refs/heads/main', deploy: 'true' },
])(
	'$event ignores an unavailable previous PR head',
	async ({ event, ref, deploy }): Promise<void> => {
		const workflow = await readFile(
			join(repositoryRoot, '.github/workflows/demistify-release.yml'),
			'utf8',
		);
		const script = workflow.match(/node --input-type=module <<'JS'\n([\s\S]*?)\n\s+JS\n/)?.[1];
		if (!script) throw new Error('Release selection script is missing.');
		const directory = await mkdtemp(join(tmpdir(), 'demistify-workflow-'));
		const output = join(directory, 'output');
		try {
			const result = spawnSync(process.execPath, ['--input-type=module', '-'], {
				cwd: repositoryRoot,
				input: script,
				encoding: 'utf8',
				env: {
					...process.env,
					BEFORE_SHA: 'f'.repeat(40),
					GITHUB_EVENT_NAME: event,
					GITHUB_REF: ref,
					GITHUB_OUTPUT: output,
				},
			});
			expect(result.status, result.stderr).toBe(0);
			expect(await readFile(output, 'utf8')).toBe(`deploy=${deploy}\n`);
		} finally {
			await rm(directory, { recursive: true, force: true });
		}
	},
);

test('main push still requires its previous version to be readable', async (): Promise<void> => {
	const workflow = await readFile(
		join(repositoryRoot, '.github/workflows/demistify-release.yml'),
		'utf8',
	);
	const script = workflow.match(/node --input-type=module <<'JS'\n([\s\S]*?)\n\s+JS\n/)?.[1];
	if (!script) throw new Error('Release selection script is missing.');
	const directory = await mkdtemp(join(tmpdir(), 'demistify-workflow-'));
	try {
		const result = spawnSync(process.execPath, ['--input-type=module', '-'], {
			cwd: repositoryRoot,
			input: script,
			encoding: 'utf8',
			env: {
				...process.env,
				BEFORE_SHA: 'f'.repeat(40),
				GITHUB_EVENT_NAME: 'push',
				GITHUB_REF: 'refs/heads/main',
				GITHUB_OUTPUT: join(directory, 'output'),
			},
		});
		expect(result.status).not.toBe(0);
		expect(result.stderr).toContain('git show');
	} finally {
		await rm(directory, { recursive: true, force: true });
	}
});
