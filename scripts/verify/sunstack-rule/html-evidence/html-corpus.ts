/** Independent source inventory for the fixed real HTML corpus; not an engine matcher. */
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import { equal, requireCondition, writeJson } from '../rulebook/rulebook-fixture-support.ts';

export const corpusRevision = '022701fba2f2b1e133980a90892cac164361ea3f';
export const corpusDirectory = 'docs/specs/2026-11-07-design-documents';
export const corpusFiles = [
	'requirements.html',
	'specification.html',
	'program-design.html',
	'verification-design.html',
	'index.html',
] as const;
export const requirementIds = [
	'R1',
	'R2',
	'R3',
	'R4',
	'R5',
	'R6',
	'R7',
	'R8',
	'R9',
	'R10',
] as const;
export const entityIds = ['E1', 'E2', 'E3', 'E4', 'E5'] as const;
export interface HtmlInventory {
	readonly ids: string[];
	readonly fragmentHrefs: string[];
	readonly rowIds: string[];
	readonly firstCells: string[];
}

function attributeValues(tag: string): Map<string, string> {
	return new Map(
		[...tag.matchAll(/\s+([^\s"'<>/=]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g)].map(
			(match) => [match[1]?.toLowerCase() ?? '', match[2] ?? match[3] ?? match[4] ?? ''],
		),
	);
}

/** Lexical oracle for this inspected corpus: ignore comments/raw scripts and escaped examples. */
export function inventoryHtml(source: string): HtmlInventory {
	const markup = source
		.replace(/<!--[\s\S]*?-->/g, '')
		.replace(/<(script|style)\b[^>]*>[\s\S]*?<\/\1\s*>/gi, '');
	const tags = [...markup.matchAll(/<[a-z][a-z0-9:_-]*\b(?:[^>"']|"[^"]*"|'[^']*')*>/gi)].map(
		(match) => match[0],
	);
	const ids: string[] = [];
	const fragmentHrefs: string[] = [];
	for (const tag of tags) {
		const attributes = attributeValues(tag);
		const identifier = attributes.get('id');
		if (identifier) ids.push(identifier);
		const href = attributes.get('href');
		if (href?.includes('#')) fragmentHrefs.push(href);
	}
	const rows = [...markup.matchAll(/<tr\b([^>]*)>([\s\S]*?)<\/tr\s*>/gi)];
	return {
		ids,
		fragmentHrefs,
		rowIds: rows.flatMap((row) => {
			const identifier = row[1]?.match(/\bid=["']([RE][0-9]+)["']/)?.[1];
			return identifier ? [identifier] : [];
		}),
		firstCells: rows.flatMap((row) => {
			const cell = row[2]?.match(/<td\b[^>]*>([\s\S]*?)<\/td\s*>/i)?.[1];
			return cell
				? [
						cell
							.replace(/<[^>]*>/g, '')
							.replace(/\s+/g, ' ')
							.trim(),
					]
				: [];
		}),
	};
}

export function snapshotRealSet(options: {
	source: string;
	destination: string;
}): Record<string, string> {
	filesystem.mkdirSync(options.destination, { recursive: true });
	const hashes: Record<string, string> = {};
	const inventories: Record<string, HtmlInventory> = {};
	for (const file of corpusFiles) {
		const result = spawnSync('git', ['show', `${corpusRevision}:${corpusDirectory}/${file}`], {
			cwd: options.source,
			timeout: 10000,
		});
		requireCondition(result.status === 0, `Cannot read pinned real HTML ${file}`);
		filesystem.writeFileSync(path.join(options.destination, file), result.stdout);
		hashes[file] = createHash('sha256').update(result.stdout).digest('hex');
		inventories[file] = inventoryHtml(result.stdout.toString('utf8'));
	}
	equal(
		inventories['specification.html']?.rowIds,
		[...entityIds, ...requirementIds],
		'Real ID row inventory changed',
	);
	equal(
		inventories['program-design.html']?.firstCells.filter((cell) => /^E[0-9]/.test(cell)),
		['E1', 'E2/E5', 'E3', 'E4'],
		'Entity binding cells changed',
	);
	equal(
		inventories['verification-design.html']?.firstCells,
		['U1; R1,R5,R6,R7,R9', 'U2; R2,R3,R5,R8', 'U3; R3,R4,R9', 'U4; R7,R10'],
		'Requirement coverage cells changed',
	);
	for (const [file, inventory] of Object.entries(inventories))
		equal(inventory.fragmentHrefs, [], `Real-set forward-fragment inventory differs in ${file}`);
	equal(
		inventories['program-design.html']?.ids,
		['document'],
		'Escaped body/template example counted as an actual element',
	);
	writeJson(path.join(path.dirname(options.destination), 'real-corpus-inventory.json'), {
		revision: corpusRevision,
		directory: corpusDirectory,
		file_sha256: hashes,
		inventories,
		manual_reverse_expectation: {
			requirements: requirementIds.map((identifier) => ({ identifier, expected: 'found' })),
			entities: entityIds.map((identifier) => ({ identifier, expected: 'found' })),
		},
		forward_fragments: 0,
		limit:
			'Source inventory and literal manually inspected binding cells; no engine behavior or new runtime pass follows from snapshot preparation.',
	});
	return hashes;
}
