import { expect, test } from 'vitest';

import { checkHtml } from './check.ts';

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
