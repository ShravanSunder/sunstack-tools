import { readdir, readFile, realpath } from 'node:fs/promises';
import { join, relative } from 'node:path';

import { readPageMarkup } from './design-page-markup.ts';

async function htmlFiles(directory: string): Promise<string[]> {
	const entries = await readdir(directory, { withFileTypes: true });
	const nested = await Promise.all(
		entries.map(async (entry): Promise<string[]> => {
			if (entry.name.startsWith('.') || entry.name === 'node_modules') return [];
			const path = join(directory, entry.name);
			if (entry.isDirectory()) return htmlFiles(path);
			return entry.isFile() && entry.name.endsWith('.html') ? [path] : [];
		}),
	);
	return nested.flat();
}

function placesRecords(html: string, file: string): boolean {
	try {
		const { placements, trees } = readPageMarkup(html, file);
		return placements.length > 0 || trees.length > 0;
	} catch {
		// A page the checker cannot read is not one it would place records on.
		return false;
	}
}

/** A page's real path: an authored path may differ from the scanned one by a link or by case. */
function pageIdentity(file: string): Promise<string> {
	return realpath(file).catch((): string => file);
}

/**
 * HTML pages under the initiative that bind design records while no design.ts or
 * initiative.ts registers them: the checker never reads them, so their records stay empty.
 */
export async function unregisteredRecordPages(
	root: string,
	registered: ReadonlySet<string>,
): Promise<string[]> {
	const registeredFiles = new Set(await Promise.all([...registered].map(pageIdentity)));
	const scanned = await htmlFiles(root);
	const scannedIdentities = await Promise.all(scanned.map(pageIdentity));
	const candidates = scanned.filter((_, index) => {
		const file = scannedIdentities[index];
		return file !== undefined && !registeredFiles.has(file);
	});
	const pages = await Promise.all(
		candidates.map(async (file): Promise<string | undefined> =>
			placesRecords(await readFile(file, 'utf8'), file) ? relative(root, file) : undefined,
		),
	);
	return pages.filter((page): page is string => page !== undefined).toSorted();
}
