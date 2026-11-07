import { expect, test } from 'vitest';

import { checkHtml } from './check.ts';

const story = `<ex-story subject="Renderer" time="Fri 08:31" icon="package" tone="done">
<details id="story-renderer" open><summary>Renderer review</summary>
<p slot="headline">A supplied headline.</p>
<div slot="happened">The supplied <strong>rich account</strong>.</div>
<div slot="for-you">Review <a href="#entry-renderer">the record</a>.</div>
<div slot="next">Observe the ordinary path.</div>
<details slot="more"><summary>More</summary><p>Optional depth.</p></details>
<details slot="record"><summary>The record: 1 entry</summary>
<details id="entry-renderer"><summary>Supplied check</summary><p>The complete supplied body.</p></details>
</details></details></ex-story>`;

function pageWithStory(storySource: string): string {
	return `<html><head><link rel="stylesheet" href="theme.css"><script type="module" src="runtime.js"></script></head><body><ex-document id="work"><h1>Shared story records</h1><h2>What it means <ex-fold-toggle for="work"></ex-fold-toggle></h2><ex-section id="done-next" title="What's done / what's next" icon="clock" series="2">${storySource}</ex-section></ex-document></body></html>`;
}

test('the complete native story and section composition checks without errors', async (): Promise<void> => {
	// Arrange: literal contract markup contains rich content, identity and native carried states.
	const source = pageWithStory(story);
	// Act.
	const findings = await checkHtml(source, { diagrams: false });
	// Assert: unknown elements or omitted required child support cannot pass.
	expect(findings.filter((finding): boolean => finding.severity === 'error')).toEqual([]);
});

test.each([
	story.replace('<p slot="headline">A supplied headline.</p>', ''),
	story.replace('<div slot="next">Observe the ordinary path.</div>', ''),
	story.replace('slot="happened"', 'slot="invented-row"'),
	story.replace('<summary>Renderer review</summary>', ''),
	story.replace(
		'</ex-story>',
		'<details><summary>Second story root</summary></details></ex-story>',
	),
	story.replace('slot="more"', 'slot="record"'),
	story.replace(
		'<details slot="more">',
		'<div slot="showpiece">First</div><div slot="showpiece">Second</div><details slot="more">',
	),
])(
	'malformed story structure produces a story-specific error',
	async (storySource): Promise<void> => {
		const findings = await checkHtml(pageWithStory(storySource), { diagrams: false });
		expect(
			findings.some((finding): boolean => finding.rule === 'story' && finding.severity === 'error'),
		).toBe(true);
	},
);

test('a zero-entry record and an explicitly closed story retain valid native markup', async (): Promise<void> => {
	const source = story
		.replace('id="story-renderer" open', 'id="story-renderer"')
		.replace('The record: 1 entry', 'The record: 0 entries')
		.replace(
			'<details id="entry-renderer"><summary>Supplied check</summary><p>The complete supplied body.</p></details>',
			'',
		)
		.replace('<a href="#entry-renderer">the record</a>', 'the record');
	const findings = await checkHtml(pageWithStory(source), { diagrams: false });
	expect(findings.filter((finding): boolean => finding.severity === 'error')).toEqual([]);
});
