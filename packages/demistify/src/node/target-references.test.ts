import { expect, test } from 'vitest';

import { checkHtml } from './check.ts';

test.each(['ex-card', 'ex-region'])(
	'%s accepts six category slots independently of tone and permits neutral surfaces',
	async (tag): Promise<void> => {
		// Arrange: the adopted category convention is 1–6; omission stays neutral.
		const surfaces = ['', '1', '2', '3', '4', '5', '6']
			.map(
				(series): string =>
					`<${tag} title="Gateway" tone="risk"${series ? ` series="${series}"` : ''}><p>Boundary · blocked.</p></${tag}>`,
			)
			.join('');
		// Act: exercise the same vocabulary validation used by the CLI.
		const findings = await checkHtml(`<ex-board>${surfaces}</ex-board>`, { diagrams: false });
		// Assert: category does not replace the required semantic tone.
		expect(findings.filter((finding): boolean => finding.rule === 'attribute')).toEqual([]);
	},
);

test.each(['', '0', '7', '1.5', 'blue'])(
	'the checker rejects category series "%s" on cards and regions',
	async (series): Promise<void> => {
		// Arrange: none of these authored values names an adopted chart slot.
		const source = `<ex-board><ex-region title="Boundary" tone="note" series="${series}"><ex-card title="Gateway" tone="note" series="${series}"></ex-card></ex-region></ex-board>`;
		// Act.
		const findings = await checkHtml(source, { diagrams: false });
		// Assert: each surface reports its invalid category at the authoring boundary.
		const categories = findings.filter(
			(finding): boolean => finding.rule === 'attribute' && finding.message.includes('series'),
		);
		expect(categories).toHaveLength(2);
		expect(categories.every((finding): boolean => finding.severity === 'error')).toBe(true);
	},
);

test.each([
	'<script type="text/plain">first\nsecond\nlast</script>',
	'<template>first\n&lt;/script&gt;\nlast</template>',
])('code notes use the actual source carrier rows', async (carrier): Promise<void> => {
	// Arrange: row 2 exists in both carriers; row 9 exists in neither.
	const source = `<ex-code lang="text">${carrier}<ex-note line="2">Second row.</ex-note><ex-note line="9">Missing row.</ex-note></ex-code>`;

	// Act: validate the authored source through the real checker.
	const findings = await checkHtml(source, { diagrams: false });

	// Assert: faithful template text can carry the same valid notes as script text.
	const notes = findings.filter((finding): boolean => finding.rule === 'note');
	expect(notes).toHaveLength(1);
	expect(notes[0]?.message).toContain('line="9"');
	expect(notes[0]?.message).toContain('rows are 1–3');
});

test('the checker accepts the two shared widths and rejects misspelled widths on native wrappers too', async (): Promise<void> => {
	const source =
		'<ex-document><ex-tabs data-width="reading"><section label="Text">First.</section></ex-tabs><ex-code data-width="canvas">code</ex-code><section data-width="narrow">Unknown width.</section></ex-document>';
	const findings = await checkHtml(source, { diagrams: false });
	const widths = findings.filter(
		(finding): boolean => finding.rule === 'attribute' && finding.message.includes('data-width'),
	);

	expect(widths).toHaveLength(1);
	expect(widths[0]?.message).toContain('reading, canvas');
});

test('a qualified diagram target must name a node in that diagram', async (): Promise<void> => {
	const source =
		'<ex-mermaid id="before"><script type="text/plain">flowchart LR\nClient --> Alpha</script></ex-mermaid><ex-mermaid id="after"><script type="text/plain">flowchart LR\nClient --> Beta</script></ex-mermaid><ex-presentation id="walk" title="Walk" description="The intended node"><ex-step to="before--Beta"></ex-step><ex-step to="after--Beta"></ex-step></ex-presentation>';
	const findings = await checkHtml(source, { diagrams: false });
	const missing = findings.filter((finding): boolean => finding.rule === 'missing-id');
	expect(missing).toHaveLength(1);
	expect(missing[0]?.message).toContain('before--Beta');
});

test('file Markdown headings are valid scoped targets, with duplicate suffixes', async (): Promise<void> => {
	const source =
		'<ex-markdown id="notes" src="notes.md"></ex-markdown><ex-presentation id="walk" title="Walk" description="Recovery"><ex-step to="notes--recovery"></ex-step><ex-step to="notes--recovery-2"></ex-step><ex-step to="notes--missing"></ex-step></ex-presentation>';
	const findings = await checkHtml(source, {
		diagrams: false,
		readSource: async (path: string): Promise<string | undefined> =>
			path === 'notes.md' ? '## Recovery\n\nFirst.\n\n## Recovery\n\nSecond.' : undefined,
	});
	const missing = findings.filter((finding): boolean => finding.rule === 'missing-id');
	expect(missing).toHaveLength(1);
	expect(missing[0]?.message).toContain('notes--missing');
});

test('Markdown diagram targets resolve in their own file and across pages', async (): Promise<void> => {
	// Two fences repeat Client but own different destinations; valid stops must not borrow nodes.
	const markdown =
		'## Architecture\n\n```mermaid\nflowchart LR\nClient --> Alpha\n```\n\n```mermaid\nflowchart LR\nClient --> Registry\n```';
	const targetPage = '<ex-markdown id="notes" src="notes.md"></ex-markdown>';
	const source = `${targetPage}<ex-presentation id="walk" title="Walk" description="Scoped diagrams"><ex-step to="notes--diagram-1--Alpha"></ex-step><ex-step to="notes--diagram-2--Registry"></ex-step><ex-step to="details.html#notes--diagram-2--Registry"></ex-step><ex-step to="notes--diagram-1--Registry"></ex-step><ex-step to="details.html#notes--diagram-1--Registry"></ex-step></ex-presentation>`;
	const findings = await checkHtml(source, {
		diagrams: false,
		readSource: async (path: string): Promise<string | undefined> =>
			path === 'notes.md' ? markdown : path === 'details.html' ? targetPage : undefined,
	});
	const missing = findings.filter((finding): boolean => finding.rule === 'missing-id');
	expect(missing).toHaveLength(2);
	expect(missing.map((finding): string | undefined => finding.message.split('"')[1])).toEqual(
		expect.arrayContaining([
			'notes--diagram-1--Registry',
			'details.html#notes--diagram-1--Registry',
		]),
	);
});
