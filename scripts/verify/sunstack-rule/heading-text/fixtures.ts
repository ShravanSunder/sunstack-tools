/** Exact P3 input copies, with the released conventions explicitly restored. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import { z } from 'zod';

import { configuredBytes } from '../config-only/configured-parity.ts';
import { objectValue, readObject, type JsonObject } from '../rulebook/rulebook-fixture-support.ts';

const groupSchema = z.object({
	name: z.enum(['html', 'markdown', 'j1', 'ai-tools']),
	config: z.string(),
	expected_baseline_requests: z.number(),
	config_sha256: z.string(),
	files: z.array(z.object({ file: z.string(), sha256: z.string() })),
});
export type CorpusGroup = z.infer<typeof groupSchema>;
export function groups(): CorpusGroup[] {
	return z
		.object({ groups: z.array(groupSchema) })
		.parse(readObject(path.join(import.meta.dirname, 'real-corpus.json'))).groups;
}
export function requireInputs(root: string): void {
	for (const group of groups()) {
		assert.equal(
			createHash('sha256')
				.update(filesystem.readFileSync(path.join(root, group.name, group.config)))
				.digest('hex'),
			group.config_sha256,
		);
		for (const file of group.files)
			assert.equal(
				createHash('sha256')
					.update(filesystem.readFileSync(path.join(root, group.name, file.file)))
					.digest('hex'),
				file.sha256,
			);
	}
}
export function projectConfigPath(common: string, group: CorpusGroup): string {
	return path.join(
		common,
		group.name === 'html' || group.name === 'markdown' ? 'rulebooks' : 'projects',
		group.name,
		group.config,
	);
}
export function prepareCopies(input: string, common: string): void {
	filesystem.mkdirSync(common);
	filesystem.mkdirSync(path.join(common, 'projects'));
	filesystem.mkdirSync(path.join(common, 'configs'));
	filesystem.mkdirSync(path.join(common, 'output'));
	filesystem.mkdirSync(path.join(common, 'rulebooks'));
	for (const group of groups()) {
		const root = path.join(common, 'projects', group.name);
		const selectionProject = group.name === 'html' || group.name === 'markdown';
		if (selectionProject) {
			filesystem.mkdirSync(root);
			filesystem.cpSync(path.join(input, group.name, 'pages'), path.join(root, 'pages'), {
				recursive: true,
			});
			filesystem.cpSync(path.join(input, group.name), path.join(common, 'rulebooks', group.name), {
				recursive: true,
			});
		} else filesystem.cpSync(path.join(input, group.name), root, { recursive: true });
		const original = filesystem.readFileSync(path.join(input, group.name, group.config));
		const bytes = group.name === 'ai-tools' ? original : configuredBytes(original);
		filesystem.writeFileSync(path.join(common, 'configs', `${group.name}.json`), bytes);
		filesystem.writeFileSync(projectConfigPath(common, group), bytes);
	}
}
export function headingConfig(original: Buffer): Buffer {
	const config = objectValue(JSON.parse(original.toString('utf8')));
	const kinds = objectValue(objectValue(config['evidence'])['kinds']);
	let changed = 0;
	for (const name of ['html_s1', 'html_s2', 'md_s1', 'md_s2']) {
		const field = objectValue(objectValue(objectValue(kinds[name])['fields'])['heading_path']);
		assert.equal(field['from'], 'enclosing');
		assert.equal(field['text'], 'first_line');
		field['text'] = 'heading';
		changed++;
	}
	assert.equal(changed, 4);
	return Buffer.from(`${JSON.stringify(config, null, 2)}\n`);
}
export function savedReport(bytes: Buffer): JsonObject {
	return objectValue(JSON.parse(bytes.toString('utf8')));
}
