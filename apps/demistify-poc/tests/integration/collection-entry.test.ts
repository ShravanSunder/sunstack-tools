import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { expect, test } from 'vitest';

import { prepareUnderstandingPages } from '../../scripts/collect-understanding-records.ts';
import { writeCollectionFixture } from '../collection-fixture.ts';

test('the real preparer accepts collection identities without fixture entry names', async (): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), '2026-11-collection-entry-'));
	await writeCollectionFixture(directory);
	const prepared = await prepareUnderstandingPages(directory);
	expect(prepared.collection.records.some(({ record }) => record.id === 'FirstNeed')).toBe(true);
	expect(
		prepared.collection.records
			.filter(({ record }) => record.kind === 'stack')
			.map(({ record }) => record.id),
	).toEqual(['help-path']);
});

test.each([
	['requirements.html', 'user need'],
	['execution.html', 'call stack'],
])('the preparer rejects a collection without its %s entry', async (file, label): Promise<void> => {
	const directory = await mkdtemp(join(tmpdir(), '2026-11-collection-entry-'));
	await writeCollectionFixture(directory);
	const path = join(directory, file);
	const html = await readFile(path, 'utf8');
	await writeFile(
		path,
		html.replace(/(<script[^>]*data-lab-records[^>]*>)[\s\S]*?(<\/script>)/, '$1[]$2'),
	);
	await expect(prepareUnderstandingPages(directory)).rejects.toThrow(`at least one ${label}`);
});
