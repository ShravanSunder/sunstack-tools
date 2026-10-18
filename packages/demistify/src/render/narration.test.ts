import { expect, test } from 'vitest';

import { renderNarrationContent } from './markdown-content.ts';

test('narration keeps readable Markdown and resolves links from the route home', (): void => {
	const source =
		'**Why two levels?**\n\n- The **cloud** connects services.\n- Inspect `SessionId`.\n\n[Read the contract](contract.html#identity)';
	const rendered = renderNarrationContent(source, 'file:///review/index.html');

	expect(rendered).toContain('<strong>Why two levels?</strong>');
	expect(rendered).toContain('<ul>');
	expect(rendered).toContain('<code>SessionId</code>');
	expect(rendered).toContain('href="file:///review/contract.html#identity"');
});

test('carried narration cannot introduce executable links, HTML or a second document hierarchy', (): void => {
	const source =
		'# A step\n\n[unsafe](javascript:alert%281%29)\n\n<img src=x onerror=alert(1)>\n\n![Evidence](capture.png)\n\n```mermaid\nflowchart LR\nA --> B\n```';
	const rendered = renderNarrationContent(source, 'https://example.test/review.html');

	expect(rendered).toContain('<p><strong>A step</strong></p>');
	expect(rendered).toContain('unsafe');
	expect(rendered).toContain('&lt;img');
	expect(rendered).toContain('Evidence');
	expect(rendered).toContain('flowchart LR');
	expect(rendered).not.toMatch(/<(?:h[1-6]|img|script|ex-mermaid)\b/);
	expect(rendered).not.toContain('href="javascript:');
});
