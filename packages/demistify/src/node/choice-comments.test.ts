import { expect, test } from 'vitest';

import { renderChoiceTitle } from '../render/markdown-content.ts';
import { checkHtml } from './check.ts';

const option =
	'<ex-option id="layout" title="**Layout** only"><template slot="summary">Review **layout**.</template></ex-option>';
const page = `<link rel="stylesheet" href="theme.css"><script type="module" src="runtime.js"></script><ex-document id="content" document-id="scope" revision="r1"><h1>Scope</h1><ex-choice id="scope-question" title="Which scope?">${option}</ex-choice></ex-document><ex-comments for="content"></ex-comments>`;

test('the public choice and identified Comments markup checks without warnings', async (): Promise<void> => {
	expect(await checkHtml(page, { diagrams: false })).toEqual([]);
});

test.each([
	page.replace('document-id="scope" revision="r1"', ''),
	page.replace('document-id="scope" revision="r1"', 'comments="off"'),
])(
	'Comments derives missing metadata and accepts the document opt-out',
	async (source): Promise<void> => {
		expect(await checkHtml(source, { diagrams: false })).toEqual([]);
	},
);

test.each([
	[page.replace('revision="r1"', 'revision=""'), 'comments'],
	[page.replace('document-id="scope"', 'document-id=" "'), 'comments'],
	[page.replace('revision="r1"', 'comments="sometimes"'), 'attribute'],
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
