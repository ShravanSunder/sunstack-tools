/** Fixed expectations from pinned Markdown, remark 11/GFM and github-slugger 2.0.0. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import { z } from 'zod';

const spanSchema = z.tuple([z.number().int().nonnegative(), z.number().int().nonnegative()]);
const fileSchema = z.object({ file: z.string(), sha256: z.string().length(64) });
const linkSchema = z.object({
	source: z.string(),
	target: z.string(),
	fragment: z.string(),
	url: z.string(),
	raw: z.string(),
	source_sha256: z.string().length(64),
	start: z.number().int(),
	end: z.number().int(),
	expected: z.enum(['found', 'missing', 'missing_file']),
});
const repositorySchema = z.object({
	key: z.string(),
	repo: z.string(),
	commit: z.string().length(40),
	files: z.array(fileSchema),
	links: z.array(linkSchema),
});
const sectionSchema = z.object({
	label: z.string(),
	level: z.number().int(),
	start: z.number().int(),
	end: z.number().int(),
	parent: z.number().int().nullable(),
	own: z.array(spanSchema),
	quote: z.boolean(),
	list: z.boolean(),
});
const sectionFileSchema = fileSchema.extend({ repo: z.string(), sections: z.array(sectionSchema) });
export type Repository = z.infer<typeof repositorySchema>;
export type Link = z.infer<typeof linkSchema>;
export type Section = z.infer<typeof sectionSchema>;
export const repositories = z
	.array(repositorySchema)
	.parse(JSON.parse(filesystem.readFileSync(new URL('./links.json', import.meta.url), 'utf8')));
export const sectionFiles = z
	.array(sectionFileSchema)
	.parse(JSON.parse(filesystem.readFileSync(new URL('./sections.json', import.meta.url), 'utf8')));

export function requireFile(root: string, file: z.infer<typeof fileSchema>): Buffer {
	const target = path.join(root, file.file);
	assert.ok(filesystem.lstatSync(target).isFile(), `Frozen input is not a regular file: ${target}`);
	const raw = filesystem.readFileSync(target);
	assert.equal(createHash('sha256').update(raw).digest('hex'), file.sha256, target);
	return raw;
}
export function requireInputs(root: string): void {
	for (const repository of repositories) {
		const directory = path.join(root, repository.key);
		for (const file of repository.files) requireFile(directory, file);
		for (const link of repository.links) {
			const raw = filesystem.readFileSync(path.join(directory, link.source));
			assert.equal(
				raw.subarray(link.start, link.end).toString('utf8'),
				link.raw,
				'Oracle offsets must identify original UTF8 link bytes',
			);
			if (link.expected === 'missing_file')
				assert.equal(filesystem.existsSync(path.join(directory, link.target)), false);
		}
	}
	for (const file of sectionFiles) requireFile(path.join(root, file.repo), file);
}
export const controlsMarkdown =
	'# Root 文\nRoot body.\n\n- Lead\n\n  ## List child\n  List body.\n\n- Peer item\n\n## Outer\nOuter body.\n\n> ### Quote child\n> Quote body.\n\n#### Deep\nDeep body.\n\n## Peer\nTail.\n';
