/** Copy only fixed Git blobs needed by the Markdown journey; original checkouts stay read-only. */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireOutsideInput } from '../git-first/frozen-inputs.ts';
import { writeJson } from '../rulebook/rulebook-fixture-support.ts';
import { controlsMarkdown, repositories, requireInputs, sectionFiles } from './oracles.ts';

const optionsSchema = z.object({ sunstack: z.string(), tools: z.string(), output: z.string() });
export function captureMarkdown(options: z.infer<typeof optionsSchema>): void {
	const output = path.resolve(options.output);
	for (const source of [options.sunstack, options.tools])
		requireOutsideInput(path.resolve(source), output);
	filesystem.mkdirSync(output);
	for (const repository of repositories) {
		const source = repository.key === 'sunstack' ? options.sunstack : options.tools;
		const names = new Set([
			...repository.files.map((file) => file.file),
			...sectionFiles.filter((file) => file.repo === repository.key).map((file) => file.file),
		]);
		for (const file of names) {
			const result = spawnSync('git', ['show', `${repository.commit}:${file}`], {
				cwd: source,
				timeout: 10000,
				maxBuffer: 4_000_000,
			});
			assert.equal(result.status, 0, `Missing fixed Git blob: ${repository.commit}:${file}`);
			const destination = path.join(output, repository.key, file);
			filesystem.mkdirSync(path.dirname(destination), { recursive: true });
			filesystem.writeFileSync(destination, result.stdout);
		}
	}
	filesystem.writeFileSync(path.join(output, 'controls.md'), controlsMarkdown);
	requireInputs(output);
	writeJson(path.join(output, 'capture.json'), {
		repositories: repositories.map((repository) => ({
			repo: repository.repo,
			commit: repository.commit,
		})),
		original_checkouts: 'Git object reads only',
		hashes_verified: true,
	});
}
if (process.argv[1] === import.meta.filename) {
	const { values } = parseArgs({
		options: {
			sunstack: { type: 'string' },
			tools: { type: 'string' },
			output: { type: 'string' },
			help: { type: 'boolean' },
		},
	});
	if (values.help)
		process.stdout.write(
			'Capture fixed real Markdown inputs from existing Git objects into a new outside directory.\n--sunstack AI_TOOLS_REPO --tools SUNSTACK_TOOLS_REPO --output NEW_DIRECTORY\nNo checkout, fetch, credentials or original-file writes.\n',
		);
	else captureMarkdown(optionsSchema.parse(values));
}
