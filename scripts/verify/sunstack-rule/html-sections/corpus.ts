/** Fixed independent outline expectations; real input bytes are supplied outside the checkout. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import { z } from 'zod';

export const sectionSchema = z.object({
	level: z.number().int().min(1).max(6),
	start: z.number().int().nonnegative(),
	end: z.number().int().positive(),
	own_end: z.number().int().positive(),
	parent: z.number().int().nonnegative().nullable(),
	heading: z.string(),
});
export type Section = z.infer<typeof sectionSchema>;
const corpusSchema = z.object({
	file: z.string(),
	sha256: z.string().regex(/^[a-f0-9]{64}$/),
	source_relative: z.string(),
	source_commit: z
		.string()
		.regex(/^[a-f0-9]{40}$/)
		.nullable(),
	sections: z.array(sectionSchema).nonempty(),
});
export const corpus = ['flat', 'mixed', 'wrapped'].map((name) =>
	corpusSchema.parse(
		JSON.parse(filesystem.readFileSync(new URL(`./corpus-${name}.json`, import.meta.url), 'utf8')),
	),
);
export function requireCorpus(input: string): void {
	for (const entry of corpus) {
		const file = path.join(input, entry.file);
		assert.ok(filesystem.lstatSync(file).isFile(), 'Corpus input must be a regular file');
		assert.equal(
			createHash('sha256').update(filesystem.readFileSync(file)).digest('hex'),
			entry.sha256,
		);
	}
}
