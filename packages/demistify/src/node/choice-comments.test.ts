import { expect, test } from 'vitest';

import { renderChoiceTitle } from '../render/markdown-content.ts';
import { checkHtml } from './check.ts';

const option =
	'<ex-option id="layout" title="**Layout** only"><template slot="summary">Review **layout**.</template></ex-option>';
const page = `<link rel="stylesheet" href="theme.css"><script type="module" src="runtime.js"></script><ex-document id="content" document-id="scope" revision="r1"><h1>Scope</h1><ex-choice id="scope-question" title="Which scope?">${option}</ex-choice></ex-document><ex-comments for="content"></ex-comments>`;

test('the public choice and identified Comments markup checks without warnings', async (): Promise<void> => {
	expect(await checkHtml(page, { diagrams: false })).toEqual([]);
});

const recordChoice = `<ex-choice>
	<template data-record-body><p>What changed since the last decision.</p></template>
	<ex-option data-option="layout"><template slot="summary">Review the layout.</template></ex-option>
</ex-choice>`;
const sectionChoice = page.replace(
	/<ex-choice.*<\/ex-choice>/u,
	`<ex-section id="open-decisions" title="For you to decide" holds="needs-owner"></ex-section>
	<ex-section-item record="project/D40">${recordChoice}</ex-section-item>`,
);

test('the standalone checker accepts a typed section choice with record-owned titles and IDs', async (): Promise<void> => {
	expect(await checkHtml(sectionChoice, { diagrams: false })).toEqual([]);
});

test.each([
	['Review the layout.', 'one nonempty template slot="summary"'],
	[' data-option="layout"', 'nonempty data-option'],
])(
	'the standalone checker retains typed option validation',
	async (removed, message): Promise<void> => {
		const findings = await checkHtml(sectionChoice.replace(removed, ''), { diagrams: false });
		expect(findings.map(({ rule }) => rule)).toEqual(['choice']);
		expect(findings[0]?.message).toContain(message);
	},
);

test('the standalone checker still requires authored titles for a plain choice', async (): Promise<void> => {
	const invalid = page.replace(' title="Which scope?"', '').replace(' title="**Layout** only"', '');
	const findings = await checkHtml(invalid, { diagrams: false });
	expect(findings.map(({ rule }) => rule)).toEqual(['attribute', 'attribute', 'choice-title']);
});

test('Comments derives missing metadata without author setup', async (): Promise<void> => {
	expect(
		await checkHtml(page.replace('document-id="scope" revision="r1"', ''), { diagrams: false }),
	).toEqual([]);
});

test.each([
	[page.replace('revision="r1"', 'revision=""'), 'comments'],
	[page.replace('document-id="scope"', 'document-id=" "'), 'comments'],
	[page.replace('revision="r1"', 'comments="sometimes"'), 'attribute'],
	[page.replace('revision="r1"', 'comments="off"'), 'attribute'],
	[page.replace('for="content"', 'for="scope-question"'), 'comments'],
	[page + '<ex-comments for="content"></ex-comments>', 'comments'],
	[page.replace('id="layout"', 'id="scope-question"'), 'duplicate-id'],
	[page.replace('<template slot="summary">Review **layout**.</template>', ''), 'choice'],
	[page.replace('title="**Layout** only"', 'title="# A heading"'), 'choice-title'],
	[page.replace('title="**Layout** only"', 'title="[Link](other.html)"'), 'choice-title'],
	[page.replace('title="Which scope?"', 'title="Which scope?" view="unknown"'), 'attribute'],
])(
	'the checker rejects malformed public choice/comment inputs',
	async (source, rule): Promise<void> => {
		const findings = await checkHtml(source, { diagrams: false });
		expect(
			findings.some((finding): boolean => finding.severity === 'error' && finding.rule === rule),
		).toBe(true);
	},
);

test('inline title parsing preserves emphasis and code, rejecting additional hierarchy', (): void => {
	expect(renderChoiceTitle('**Layout** with `scope`')).toBe(
		'<strong>Layout</strong> with <code>scope</code>',
	);
	for (const invalid of [
		'',
		'# Heading',
		'First\n\nSecond',
		'- List',
		'[Link](https://example.test)',
	])
		expect((): string => renderChoiceTitle(invalid)).toThrow();
});
