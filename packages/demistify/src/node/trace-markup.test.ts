import { expect, test } from 'vitest';

import { checkHtml } from './check.ts';

const pageHead =
	'<head><title>Trace markup</title><link rel="stylesheet" href="theme.css"><script type="module" src="runtime.js"></script></head>';

test.each(['ex-trace', 'ex-timeline'])(
	'%s accepts only named disclosure steps',
	async (tag): Promise<void> => {
		const wrap = (content: string): string =>
			`<!doctype html><html lang="en">${pageHead}<body><ex-document><h1>Trace markup</h1><${tag} title="Run evidence" zoom>${content}</${tag}></ex-document></body></html>`;
		await expect(
			checkHtml(
				wrap(
					'<details open tone="done" tag="Observed"><summary>Review outcomes</summary><p>Evidence.</p></details>',
				),
			),
		).resolves.toEqual([]);
		const bad = await checkHtml(
			wrap('<section>Old event body</section><details><p>Missing summary</p></details>'),
		);
		expect(bad.filter(({ rule }): boolean => rule === 'trace')).toHaveLength(2);
	},
);

test('the checker rejects retired custom elements', async (): Promise<void> => {
	const tags = ['ex-zoom', 'ex-decision', 'ex-diagram-diff', 'ex-layer'];
	const html = `<!doctype html><html lang="en">${pageHead}<body><ex-document><h1>Trace markup</h1>${tags.map((tag): string => `<${tag}></${tag}>`).join('')}</ex-document></body></html>`;
	const findings = await checkHtml(html);
	for (const tag of tags)
		expect(
			findings.some(
				({ rule, message, severity }): boolean =>
					rule === 'unknown-element' &&
					severity === 'error' &&
					message.startsWith(`Unknown element <${tag}>.`),
			),
		).toBe(true);
});
