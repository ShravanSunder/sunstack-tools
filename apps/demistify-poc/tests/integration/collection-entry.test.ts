import { createHash } from 'node:crypto';
import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { prepareDesignPages } from '@sunstack/demistify/design-collection';
import { parse, type DefaultTreeAdapterMap } from 'parse5';
import { expect, test } from 'vitest';
import { z } from 'zod';

import { checkHtml } from '../../../../packages/demistify/src/node/check.ts';
import { collectDesignRecords } from '../../../../packages/demistify/src/node/design-markup.ts';
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

const cedarCurrent = new URL('../../experiments/agent-written/cedar-documents/', import.meta.url);
const cedarFrozen = new URL('../fixtures/cedar-author-output/', import.meta.url);
const cedarPages = [
	'requirements.html',
	'specification.html',
	'program-design.html',
	'verification.html',
	'execution.html',
];

test('the frozen Cedar author output retains its exact source bytes and provenance', async (): Promise<void> => {
	const manifest = z
		.object({
			sourceCommit: z.string(),
			files: z.array(z.object({ path: z.string(), sha256: z.string() })),
		})
		.parse(JSON.parse(await readFile(new URL('frozen-sha256.json', cedarFrozen), 'utf8')));
	expect(manifest.sourceCommit).toBe('b8bd05bd6a0462cac4f6a215ac55d6b25f01c5bc');
	expect(manifest.files).toHaveLength(9);
	await Promise.all(
		manifest.files.map(async (file): Promise<void> => {
			const bytes = await readFile(new URL(`references/${file.path}`, cedarFrozen));
			expect(createHash('sha256').update(bytes).digest('hex'), file.path).toBe(file.sha256);
		}),
	);
	const provenance = z
		.object({ files: z.array(z.object({ name: z.string(), copySha256: z.string() })) })
		.parse(JSON.parse(await readFile(new URL('references/provenance.json', cedarFrozen), 'utf8')));
	for (const file of provenance.files)
		expect(manifest.files.find((item) => item.path === file.name)?.sha256).toBe(file.copySha256);
});

function visibleCedarElements(
	node: DefaultTreeAdapterMap['node'],
): readonly DefaultTreeAdapterMap['element'][] {
	if ('tagName' in node && node.tagName === 'template') return [];
	return [
		...('tagName' in node ? [node] : []),
		...('childNodes' in node ? node.childNodes.flatMap(visibleCedarElements) : []),
	];
}

test('current Cedar preserves authored facts while every body is visible native HTML', async (): Promise<void> => {
	await Promise.all(
		cedarPages.map(async (pageName): Promise<void> => {
			const original = await readFile(new URL(`references/${pageName}`, cedarFrozen), 'utf8');
			const current = await readFile(new URL(pageName, cedarCurrent), 'utf8');
			const originalRecords = collectDesignRecords(original, pageName);
			const currentRecords = collectDesignRecords(current, pageName);
			expect(currentRecords, pageName).toEqual(originalRecords);
			const elements = visibleCedarElements(parse(current));
			for (const { record } of currentRecords) {
				if (!('bodyRef' in record)) continue;
				expect(
					elements.some((element) =>
						element.attrs.some(
							(attribute) => attribute.name === 'id' && attribute.value === record.bodyRef,
						),
					),
					`${pageName}/${record.id}`,
				).toBe(true);
			}
			expect(current).not.toContain('<ex-requirements');
			const errors = await checkHtml(current, { diagrams: false });
			expect(
				errors.filter((finding) => finding.severity === 'error'),
				pageName,
			).toEqual([]);
		}),
	);
	const prepared = await prepareDesignPages(cedarCurrent.pathname);
	expect(
		prepared.collection.records
			.filter(({ record }) => record.kind === 'need')
			.map(({ record }) => record.id),
	).toEqual(['CedarKeepDraft', 'CedarExplainSkip']);
});
