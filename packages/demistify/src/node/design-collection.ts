import { createHash } from 'node:crypto';
import { readFile, readdir } from 'node:fs/promises';
import { join, resolve } from 'node:path';

import { parse, serialize } from 'parse5';

import { type PreparedCollection } from '../design-documents/collection.ts';
import {
	collectDesignRecords,
	designOwnerKinds,
	designRecordReferences,
	stampPreparedDesignPage,
} from './design-markup.ts';
import { embedMarkdownFiles, markdownFileReferences } from './markdown-files.ts';

export interface PreparedDesignPages {
	readonly collection: PreparedCollection;
	readonly pages: ReadonlyMap<string, string>;
}

/** Derive all output before the caller writes; the original HTML/Markdown remains authored input. */
export async function prepareDesignPages(inputDirectory: string): Promise<PreparedDesignPages> {
	const authoredPages = new Map(
		await Promise.all(
			(await readdir(inputDirectory))
				.filter((file) => file.endsWith('.html'))
				.toSorted((left, right) => {
					const owners = Object.keys(designOwnerKinds);
					const leftIndex = owners.indexOf(left),
						rightIndex = owners.indexOf(right);
					return (
						(leftIndex < 0 ? owners.length : leftIndex) -
							(rightIndex < 0 ? owners.length : rightIndex) || left.localeCompare(right)
					);
				})
				.map(async (page): Promise<readonly [string, string]> => [
					page,
					await readFile(join(inputDirectory, page), 'utf8'),
				]),
		),
	);
	if (!authoredPages.has('requirements.html'))
		throw new Error('Missing record owner page requirements.html');
	const pageRecords = await Promise.all(
		[...authoredPages].map(async ([page, html]) => {
			const sources: Record<string, string> = {};
			await Promise.all(
				markdownFileReferences(html).map(async (reference): Promise<void> => {
					if (/^[a-z][a-z0-9+.-]*:|^\/\//i.test(reference))
						throw new Error(`${page}: prepare reads local Markdown files only: ${reference}`);
					sources[reference] = await readFile(resolve(inputDirectory, reference), 'utf8');
				}),
			);
			// The existing embedder's fast guard expects canonical src="…" spelling.
			// Normalize only the derivation copy; the written authored page stays byte-preserved.
			const derivation = Object.keys(sources).length ? serialize(parse(html)) : html;
			return collectDesignRecords(embedMarkdownFiles(derivation, sources), page);
		}),
	);
	const records = pageRecords.flat();
	const byId = new Map(records.map((item) => [item.record.id, item]));
	if (byId.size !== records.length) {
		const seen = new Set<string>();
		for (const { record, page } of records) {
			if (seen.has(record.id)) throw new Error(`${page}: Duplicate record ${record.id}`);
			seen.add(record.id);
		}
	}
	if (!records.some(({ record }) => record.kind === 'need'))
		throw new Error('A reader collection needs at least one user need.');
	for (const { record } of records)
		for (const [target, kind] of designRecordReferences(record)) {
			if (byId.get(target)?.record.kind !== kind)
				throw new Error(`${record.id}: missing or wrong-kind ${kind} reference ${target}`);
		}
	const identity = createHash('sha256').update(JSON.stringify(records)).digest('hex');
	const collection: PreparedCollection = { version: 1, identity, records };
	const carrier = `<script type="application/json" data-ex-design-collection>\n${JSON.stringify(collection).replaceAll('<', '\\u003c')}\n</script>`;
	const pages = new Map(
		[...authoredPages].map(
			([page, html]) =>
				[page, stampPreparedDesignPage({ html, page, identity, carrier })] satisfies [
					string,
					string,
				],
		),
	);
	return { collection, pages };
}
