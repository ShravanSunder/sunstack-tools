import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { prepareDesignPages } from '@sunstack/demistify/design-collection';
import { expect, test } from 'vitest';

import { writeCollectionFixture } from '../collection-fixture.ts';

test('a needs-only collection does not require fabricated call paths', async (): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), '2026-11-collection-partial-'));
	await writeCollectionFixture(directory);
	await Promise.all(
		['specification.html', 'program-design.html', 'verification.html', 'execution.html'].map(
			async (file): Promise<void> => {
				const path = join(directory, file);
				const html = await readFile(path, 'utf8');
				await writeFile(
					path,
					html
						.replace(/(<script[^>]*data-ex-records[^>]*>)[\s\S]*?(<\/script>)/, '$1[]$2')
						.replace(/\sdata-ex-part="[^"]*"/g, ''),
				);
			},
		),
	);
	const prepared = await prepareDesignPages(directory);
	expect(prepared.collection.records.map(({ record }) => record.kind)).toEqual(
		Array(6).fill('need'),
	);
});

test('the real preparer accepts collection identities without fixture entry names', async (): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), '2026-11-collection-entry-'));
	await writeCollectionFixture(directory);
	const prepared = await prepareDesignPages(directory);
	expect(prepared.collection.records.some(({ record }) => record.id === 'FirstNeed')).toBe(true);
	expect(
		prepared.collection.records
			.filter(({ record }) => record.kind === 'stack')
			.map(({ record }) => record.id),
	).toEqual(['help-path']);
});

test('the preparer rejects a collection without its user need entry', async (): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), '2026-11-collection-entry-'));
	await writeCollectionFixture(directory);
	const path = join(directory, 'requirements.html');
	const html = await readFile(path, 'utf8');
	await writeFile(
		path,
		html.replace(/(<script[^>]*data-ex-records[^>]*>)[\s\S]*?(<\/script>)/, '$1[]$2'),
	);
	await expect(prepareDesignPages(directory)).rejects.toThrow('at least one user need');
});

test('a complete collection may omit call paths while declared broken edges still reject', async (): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), '2026-11-collection-no-calls-'));
	await writeCollectionFixture(directory);
	const path = join(directory, 'execution.html');
	const html = await readFile(path, 'utf8');
	await writeFile(
		path,
		html.replace(/(<script[^>]*data-ex-records[^>]*>)[\s\S]*?(<\/script>)/, '$1[]$2'),
	);
	const prepared = await prepareDesignPages(directory);
	expect(prepared.collection.records.filter(({ record }) => record.kind === 'stack')).toEqual([]);
	const rules = join(directory, 'specification.html');
	await writeFile(rules, (await readFile(rules, 'utf8')).replace('FirstNeed', 'MissingNeed'));
	await expect(prepareDesignPages(directory)).rejects.toThrow('need reference MissingNeed');
});
