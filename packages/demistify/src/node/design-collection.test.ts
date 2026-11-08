import { execFile } from 'node:child_process';
import { mkdtemp, readFile, readdir, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { promisify } from 'node:util';

import { parse } from 'parse5';
import { expect, test } from 'vitest';

import { prepareDesignPages } from './design-collection.ts';

const runFile = promisify(execFile);

test('preparation uses real head/body boundaries and preserves literal closing tags', async (): Promise<void> => {
	const directory = await inputCollection();
	const file = join(directory, 'requirements.html');
	const source = (await readFile(file, 'utf8'))
		.replace('<title>Draft</title>', '<!-- literal </head> stays here --><title>Draft</title>')
		.replace(
			'<ex-document',
			'<!-- literal </body> stays here --><script type="text/plain">Literal </body> in inert source</script><ex-document',
		);
	await writeFile(file, source);
	const prepared = await prepareDesignPages(directory);
	const output = prepared.pages.get('requirements.html') ?? '';
	const document = parse(output);
	const html = document.childNodes.find((node) => 'tagName' in node && node.tagName === 'html');
	if (!html || !('childNodes' in html)) throw new Error('Missing html');
	const body = html.childNodes.find((node) => 'tagName' in node && node.tagName === 'body');
	if (!body || !('childNodes' in body)) throw new Error('Missing body');
	const carriers = body.childNodes.filter(
		(node) =>
			'attrs' in node &&
			node.attrs.some((attribute) => attribute.name === 'data-ex-design-collection'),
	);
	expect(carriers).toHaveLength(1);
	expect(output).toContain('<!-- literal </head> stays here -->');
	expect(output).toContain('<!-- literal </body> stays here -->');
	expect(output).toContain('<script type="text/plain">Literal </body> in inert source</script>');
});

const need = {
	kind: 'need',
	id: 'DraftNeed',
	title: 'Keep draft',
	capability: 'Editing',
	bodyRef: 'need-body',
	summaryRef: 'need-summary',
};
function page(
	records: readonly unknown[],
	body: string,
	reader = '<ex-requirements id="reader"></ex-requirements>',
): string {
	return `<!doctype html><html><head><title>Draft</title><link rel="stylesheet" href="theme.css"><script type="module" src="runtime.js"></script></head><body><ex-document id="content"><h1>Draft</h1>${reader}</ex-document><script type="application/json" data-ex-records>${JSON.stringify(records)}</script>${body}</body></html>`;
}
async function inputCollection(): Promise<string> {
	const directory = await mkdtemp(join(tmpdir(), '2026-11-design-collection-'));
	await writeFile(
		join(directory, 'requirements.html'),
		page(
			[need],
			'<template id="need-body"><p id="need-summary">Keep the draft after refusal.</p></template>',
		),
	);
	return directory;
}

async function htmlBytes(directory: string): Promise<ReadonlyMap<string, string>> {
	const names = (await readdir(directory)).filter((name) => name.endsWith('.html')).toSorted();
	return new Map(
		await Promise.all(
			names.map(async (name): Promise<[string, string]> => [
				name,
				await readFile(join(directory, name), 'utf8'),
			]),
		),
	);
}

async function addOrdinaryHome(directory: string): Promise<void> {
	await writeFile(
		join(directory, 'index.html'),
		'<!doctype html><html><head><title>Home</title><link rel="stylesheet" href="theme.css"><script type="module" src="runtime.js"></script></head><body><ex-document id="home"><h1>Purpose</h1><p>Ordinary source remains intact.</p></ex-document></body></html>',
	);
}

test('needs-only preparation is idempotent and preserves unrelated HTML', async (): Promise<void> => {
	const directory = await inputCollection();
	const home =
		'<!doctype html><html><head><title>Home</title></head><body><ex-document id="home"><h1>Purpose</h1><p>Ordinary facts.</p></ex-document></body></html>';
	await writeFile(join(directory, 'index.html'), home);
	const first = await prepareDesignPages(directory);
	expect(first.collection.records.map(({ record }) => record.id)).toEqual(['DraftNeed']);
	expect(first.collection.version).toBe(1);
	expect(first.pages.get('index.html')).toContain('<p>Ordinary facts.</p>');
	await Promise.all([...first.pages].map(([file, html]) => writeFile(join(directory, file), html)));
	const second = await prepareDesignPages(directory);
	expect([...second.pages]).toEqual([...first.pages]);
	expect(second.collection).toEqual(first.collection);
});

test('local Markdown stays authored through two derivations and changes identity', async (): Promise<void> => {
	const directory = await inputCollection();
	const authored = page(
		[need],
		'<template id="need-body"><p id="need-summary">Keep the draft after refusal.</p><ex-markdown src="notes.md"></ex-markdown></template>',
	);
	await writeFile(join(directory, 'requirements.html'), authored);
	await writeFile(join(directory, 'notes.md'), 'First **source**.');
	const first = await prepareDesignPages(directory);
	await Promise.all([...first.pages].map(([file, html]) => writeFile(join(directory, file), html)));
	await writeFile(join(directory, 'notes.md'), 'Second *source*.');
	const second = await prepareDesignPages(directory);
	expect(second.collection.identity).not.toBe(first.collection.identity);
	expect(first.collection.records[0]?.bodyHtml).toContain('First **source**.');
	expect(second.collection.records[0]?.bodyHtml).toContain('Second *source*.');
	expect(second.pages.get('requirements.html')).toContain(
		'<ex-markdown src="notes.md"></ex-markdown>',
	);
	await Promise.all(
		[...second.pages].map(([file, html]) => writeFile(join(directory, file), html)),
	);
	await writeFile(join(directory, 'notes.md'), 'Third **source**.');
	const third = await prepareDesignPages(directory);
	expect(
		new Set([first.collection.identity, second.collection.identity, third.collection.identity])
			.size,
	).toBe(3);
	expect(third.collection.records[0]?.bodyHtml).toContain('Third **source**.');
	expect(third.pages.get('requirements.html')).toContain(
		'<ex-markdown src="notes.md"></ex-markdown>',
	);
	expect(await readFile(join(directory, 'requirements.html'), 'utf8')).toContain(
		'<ex-markdown src="notes.md"></ex-markdown>',
	);
});

test.each([
	['unknown part', '<p data-ex-part="guess">Guess.</p>', 'part guess'],
	['forbidden part', '<p data-ex-part="rule">Rule.</p>', 'part rule'],
	[
		'unowned part',
		'</template><p data-ex-part="cases">Cases.</p><template>',
		'outside an owned record body',
	],
])('rejects %s before deriving output', async (_name, markup, message): Promise<void> => {
	const directory = await inputCollection();
	const source = page(
		[need],
		`<template id="need-body"><p id="need-summary">Keep the draft.</p>${markup}</template>`,
	);
	await writeFile(join(directory, 'requirements.html'), source);
	await expect(prepareDesignPages(directory)).rejects.toThrow(message);
	expect(await readFile(join(directory, 'requirements.html'), 'utf8')).toBe(source);
});

test('declared wrong-kind and missing edges remain errors with zero stacks', async (): Promise<void> => {
	const directory = await inputCollection();
	const rule = {
		kind: 'specification',
		id: 'Rule',
		title: 'Refusal',
		needs: ['MissingNeed'],
		bodyRef: 'rule-body',
		summaryRef: 'rule-summary',
	};
	await writeFile(
		join(directory, 'specification.html'),
		page(
			[rule],
			'<template id="rule-body"><p id="rule-summary">Refusal keeps the draft.</p></template>',
		),
	);
	await expect(prepareDesignPages(directory)).rejects.toThrow(
		'Rule: missing or wrong-kind need reference MissingNeed',
	);
});

test('the actual CLI prepares, checks, repeats and refuses invalid input without touching prior output', async (): Promise<void> => {
	const directory = await inputCollection();
	await addOrdinaryHome(directory);
	const cli = new URL('../../bin/demistify.ts', import.meta.url).pathname;
	const output = await runFile(process.execPath, [cli, 'prepare', directory]);
	expect(output.stdout).toContain('1 records');
	const file = join(directory, 'requirements.html');
	const first = await readFile(file, 'utf8');
	expect((await runFile(process.execPath, [cli, 'check', '--no-diagrams', file])).stdout).toContain(
		'0 errors',
	);
	await runFile(process.execPath, [cli, 'prepare', directory]);
	expect(await readFile(file, 'utf8')).toBe(first);
	await writeFile(
		join(directory, 'specification.html'),
		page(
			[
				{
					kind: 'specification',
					id: 'WrongRule',
					title: 'Wrong',
					needs: ['MissingNeed'],
					bodyRef: 'wrong-body',
					summaryRef: 'wrong-summary',
				},
			],
			'<template id="wrong-body"><p id="wrong-summary">Wrong target.</p></template>',
		),
	);
	const beforeInvalidAttempt = await htmlBytes(directory);
	await expect(runFile(process.execPath, [cli, 'prepare', directory])).rejects.toThrow(
		'need reference MissingNeed',
	);
	expect(await readFile(file, 'utf8')).toBe(first);
	expect(await htmlBytes(directory)).toEqual(beforeInvalidAttempt);
});

test.each([
	[
		'duplicate records',
		(html: string): string =>
			html
				.replace(
					JSON.stringify(need),
					`${JSON.stringify(need)},${JSON.stringify({ ...need, bodyRef: 'second-body', summaryRef: 'second-summary' })}`,
				)
				.replace(
					'</body>',
					'<template id="second-body"><p id="second-summary">Another authored body.</p></template></body>',
				),
		'Duplicate record',
	],
	[
		'missing local Markdown',
		(html: string): string =>
			html.replace('</template>', '<ex-markdown src="missing.md"></ex-markdown></template>'),
		'ENOENT',
	],
	[
		'remote Markdown',
		(html: string): string =>
			html.replace(
				'</template>',
				'<ex-markdown src="https://example.org/notes.md"></ex-markdown></template>',
			),
		'local Markdown files only',
	],
	[
		'unowned summary',
		(html: string): string => html.replace('"summaryRef":"need-summary"', '"summaryRef":"content"'),
		'missing or unowned body/summary',
	],
])(
	'the CLI rejects %s without any input/output mutation',
	async (_name, change, message): Promise<void> => {
		const directory = await inputCollection();
		await addOrdinaryHome(directory);
		const cli = new URL('../../bin/demistify.ts', import.meta.url).pathname;
		await runFile(process.execPath, [cli, 'prepare', directory]);
		const file = join(directory, 'requirements.html');
		const invalid = change(await readFile(file, 'utf8'));
		await writeFile(file, invalid);
		const beforeInvalidAttempt = await htmlBytes(directory);
		await expect(runFile(process.execPath, [cli, 'prepare', directory])).rejects.toThrow(message);
		expect(await readFile(file, 'utf8')).toBe(invalid);
		expect(await htmlBytes(directory)).toEqual(beforeInvalidAttempt);
	},
);

test('valid HTML whitespace around a local Markdown src preserves source and embeds its contents', async (): Promise<void> => {
	const directory = await inputCollection();
	const file = join(directory, 'requirements.html');
	const source = (await readFile(file, 'utf8')).replace(
		'</template>',
		'<ex-markdown src = "spaced.md"></ex-markdown></template>',
	);
	await writeFile(file, source);
	await writeFile(join(directory, 'spaced.md'), 'Actual **spaced-attribute** Markdown content.');
	const prepared = await prepareDesignPages(directory);
	expect(prepared.collection.records[0]?.bodyHtml).toContain(
		'Actual **spaced-attribute** Markdown content.',
	);
	expect(prepared.pages.get('requirements.html')).toContain(
		'<ex-markdown src = "spaced.md"></ex-markdown>',
	);
});
