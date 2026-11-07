import { expect, test } from 'vitest';

import { checkHtml } from './check.ts';

const map = `<ex-map for="work"><ex-tree><script type="text/markdown">
- [Needs you](#for-you)
- folded: Where the work is
  - planned: [Input part](#entry-input)
- [Delivery](#delivery)
- [Changes](#changes)
- [Verified](#verification)
</script></ex-tree></ex-map>`;

function pageSource(mapSource: string): string {
	return `<html><head><link rel="stylesheet" href="theme.css"><script type="module" src="runtime.js"></script></head><body>
	<ex-document id="work"><h1>Work</h1><ex-section id="for-you" title="For you"></ex-section>
	<ex-section id="delivery" title="Delivery"></ex-section><ex-section id="changes" title="Changes"></ex-section>
	<ex-section id="verification" title="Verification"></ex-section><p id="entry-input">Input</p>${mapSource}</ex-document></body></html>`;
}

test('Map checks its authored tree against actual scope sections without treating Needs you as a fifth group', async (): Promise<void> => {
	const findings = await checkHtml(pageSource(map), { diagrams: false });
	expect(findings.filter((finding): boolean => finding.severity === 'error')).toEqual([]);
});

test.each([
	['missing branch', map.replace('- [Changes](#changes)', '')],
	[
		'duplicate branch',
		map.replace('- [Changes](#changes)', '- [Changes](#changes)\n- [Again](#changes)'),
	],
	['authored total', map.replace('[Delivery]', '[Delivery · 9]')],
	['missing root', map.replace('for="work"', 'for="not-supplied"')],
	['second Map', `${map}${map}`],
	['wrong child', map.replace('<ex-tree>', '<div>').replace('</ex-tree>', '</div>')],
])('Map diagnoses %s', async (_name, source): Promise<void> => {
	const findings = await checkHtml(pageSource(source), { diagrams: false });
	expect(
		findings.some((finding): boolean => finding.rule === 'map' && finding.severity === 'error'),
	).toBe(true);
});

test.each([
	['missing tree target', map.replace('#entry-input', '#not-supplied')],
	['malformed tree target', map.replace('#entry-input', '#bad%escape')],
	['missing native target', `${map}<a href="#not-supplied">Source</a>`],
])('new composition diagnoses %s', async (_name, source): Promise<void> => {
	const findings = await checkHtml(pageSource(source), { diagrams: false });
	expect(
		findings.some(
			(finding): boolean => finding.rule === 'fragment' && finding.severity === 'error',
		),
	).toBe(true);
});

test('Map leaves other-page fragments outside same-page validation and accepts encoded stable targets', async (): Promise<void> => {
	const source = map
		.replace('#entry-input', '#%65ntry-input')
		.replace('- [Delivery]', '- [Elsewhere](other.html#not-supplied)\n- [Delivery]');
	const findings = await checkHtml(pageSource(source), { diagrams: false });
	expect(findings.filter((finding): boolean => finding.severity === 'error')).toEqual([]);
});

test('a deployment-dependent section link gets an explicit static limit rather than a false missing-branch error', async (): Promise<void> => {
	const findings = await checkHtml(
		pageSource(map.replace('#delivery', 'https://example.invalid/work.html#delivery')),
		{ diagrams: false },
	);
	expect(findings.filter((finding): boolean => finding.severity === 'error')).toEqual([]);
	expect(
		findings.some((finding): boolean => finding.rule === 'map' && finding.severity === 'warning'),
	).toBe(true);
});
