import { expect, test } from 'vitest';

import { checkHtml } from './check.ts';

const gallery =
	'<ex-gallery label="Evidence"><figure><img src="screen.svg" alt="Saved screen"></figure></ex-gallery>';

test.each([
	gallery,
	`<h1>Evidence</h1>${gallery}`,
	`<h1>Evidence</h1><script type="application/json">{}</script>${gallery}`,
	`<h1>Evidence</h1><template><ex-document>${gallery}</ex-document></template>${gallery}`,
])(
	'a dedicated gallery document accepts its optional title and inert data',
	async (content): Promise<void> => {
		const findings = await checkHtml(`<ex-document>${content}</ex-document>`, { diagrams: false });
		expect(findings.filter((finding): boolean => finding.rule === 'gallery-page')).toEqual([]);
	},
);

test.each([
	gallery,
	`<ex-document>${gallery}</ex-document><ex-document><p>Another document.</p></ex-document>`,
])('a gallery requires exactly one owning document', async (source): Promise<void> => {
	const findings = await checkHtml(source, { diagrams: false });
	expect(findings.filter((finding): boolean => finding.rule === 'gallery-page')).toEqual([
		expect.objectContaining({ severity: 'error', rule: 'gallery-page' }),
	]);
});

test.each([
	`<p>Read this explanation.</p>${gallery}`,
	`Bare Markdown explanation.\n${gallery}`,
	`<ex-compare><section label="Current">${gallery}</section></ex-compare>`,
	`<section>${gallery}</section>`,
	`${gallery}${gallery}`,
])(
	'the checker rejects embedded or multiple galleries with a dedicated-page repair',
	async (content): Promise<void> => {
		const findings = await checkHtml(`<ex-document>${content}</ex-document>`, { diagrams: false });
		const galleryFindings = findings.filter((finding): boolean => finding.rule === 'gallery-page');
		expect(galleryFindings.length).toBeGreaterThan(0);
		for (const finding of galleryFindings) {
			expect(finding.severity).toBe('error');
			expect(finding.message).toContain('dedicated');
			expect(finding.message).toContain('<figure>');
		}
	},
);
