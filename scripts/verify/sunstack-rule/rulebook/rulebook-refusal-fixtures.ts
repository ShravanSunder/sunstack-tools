/** Independent R24 input mutations, preserved from the reviewed fixture corpus. */
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import path from 'node:path';

import { z } from 'zod';

import {
	configureEndpoint,
	jsonValueSchema,
	objectValue,
	prepareFixture,
	requireCondition,
	readObject,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';

const editSchema = z.discriminatedUnion('operation', [
	z.object({
		operation: z.literal('set'),
		file: z.string(),
		path: z.array(z.string()).min(1),
		value: jsonValueSchema,
	}),
	z.object({ operation: z.literal('delete'), file: z.string(), path: z.array(z.string()).min(1) }),
	z.object({ operation: z.literal('text'), file: z.string(), text: z.string() }),
]);
const caseSchema = z.object({
	name: z.string(),
	file: z.string(),
	offending: z.string(),
	reason: z.string(),
	edits: z.array(editSchema).min(1),
});
export type RefusalCase = z.infer<typeof caseSchema>;
export function refusalCases(): RefusalCase[] {
	const raw: unknown = JSON.parse(
		filesystem.readFileSync(
			path.join(import.meta.dirname, '../fixtures/rulebook-refusal-mutations.json'),
			'utf8',
		),
	);
	return z.array(caseSchema).parse(raw);
}
function editObject(document: JsonObject, edit: z.infer<typeof editSchema>): void {
	requireCondition(edit.operation !== 'text', 'Text edits do not modify JSON objects');
	let owner = document;
	for (const key of edit.path.slice(0, -1)) owner = objectValue(owner[key]);
	const key = edit.path.at(-1);
	requireCondition(key, 'JSON edit needs a key');
	if (edit.operation === 'set') owner[key] = edit.value;
	else delete owner[key];
}
export function prepareCases(destination: string): RefusalCase[] {
	filesystem.mkdirSync(destination);
	const cases = refusalCases();
	for (const entry of cases) {
		const root = prepareFixture(path.join(destination, entry.name));
		configureEndpoint(root, 'http://127.0.0.1:9/systemone');
		for (const edit of entry.edits) {
			const file = path.join(root, edit.file);
			if (edit.operation === 'text') {
				filesystem.mkdirSync(path.dirname(file), { recursive: true });
				filesystem.writeFileSync(file, edit.text);
			} else {
				const document = readObject(file);
				editObject(document, edit);
				writeJson(file, document);
			}
		}
	}
	writeJson(path.join(destination, 'manifest.json'), cases);
	return cases;
}
export function prepareFifo(destination: string): string {
	const root = prepareFixture(destination);
	const fifo = spawnSync('mkfifo', [path.join(root, 'blocked.md')], { encoding: 'utf8' });
	requireCondition(fifo.status === 0, `Cannot prepare FIFO: ${fifo.stderr || fifo.error?.message}`);
	const config = readObject(path.join(root, 'skill-rules.config.json'));
	config['check'] = { files: ['blocked.md'] };
	writeJson(path.join(root, 'skill-rules.config.json'), config);
	const git = spawnSync('git', ['rev-parse', '--is-inside-work-tree'], {
		cwd: root,
		encoding: 'utf8',
		env: {
			PATH: '/usr/bin:/bin',
			HOME: path.join(path.dirname(root), 'git-home'),
			GIT_CONFIG_NOSYSTEM: '1',
			GIT_CONFIG_GLOBAL: '/dev/null',
		},
	});
	requireCondition(
		git.status === 128 && git.stderr.includes('not a git repository'),
		'FIFO witness must be conclusively outside Git',
	);
	return root;
}
