import { expect, test } from 'vitest';

import { checkHtml } from './check.ts';

const page =
	'<!doctype html><html><head><link rel="stylesheet" href="theme.css"><script type="module" src="runtime.js"></script></head><body><ex-nav src="./navigation/catalog.json" for="content"></ex-nav><ex-document id="content"><h1>Work</h1></ex-document></body></html>';

test('the checker rejects a missing or malformed shared navigation source', async (): Promise<void> => {
	const missing = await checkHtml(page, {
		diagrams: false,
		readSource: async (): Promise<undefined> => undefined,
	});
	expect(
		missing.some(
			(finding): boolean => finding.rule === 'shared-navigation' && finding.severity === 'error',
		),
	).toBe(true);
	const malformed = await checkHtml(page, {
		diagrams: false,
		readSource: async (): Promise<string> => '{"version":999}',
	});
	expect(
		malformed.some(
			(finding): boolean => finding.rule === 'shared-navigation' && finding.severity === 'error',
		),
	).toBe(true);
});

test('the checker follows shared relative page addresses and rejects a missing destination fragment', async (): Promise<void> => {
	const navigation = JSON.stringify({
		version: 1,
		pages: [{ id: 'peer', title: 'Peer', href: '../pages/peer.html', icon: 'path' }],
		map: [
			{
				title: 'Lost',
				description: 'Find the missing target.',
				icon: 'user',
				target: { page: 'peer', fragment: 'missing' },
			},
		],
	});
	const sources: Readonly<Record<string, string>> = {
		'./navigation/catalog.json': navigation,
		'pages/peer.html': '<h1 id="real">Peer</h1>',
	};
	const findings = await checkHtml(page, {
		diagrams: false,
		readSource: async (reference): Promise<string | undefined> => sources[reference],
	});
	expect(
		findings.some(
			(finding): boolean =>
				finding.rule === 'shared-navigation' && finding.message.includes('missing'),
		),
	).toBe(true);
});

test('the checker rejects two authored navigation owners even when the shared lists are empty', async (): Promise<void> => {
	const navigation = JSON.stringify({
		version: 1,
		pages: [{ id: 'peer', title: 'Peer', href: '../pages/peer.html', icon: 'path' }],
	});
	const sources: Readonly<Record<string, string>> = {
		'./navigation/catalog.json': navigation,
		'pages/peer.html': '<h1>Peer</h1>',
	};
	const mixed = page.replace(
		'</ex-nav>',
		'<nav slot="pages"><a href="peer.html">Peer</a></nav></ex-nav>',
	);
	const findings = await checkHtml(mixed, {
		diagrams: false,
		readSource: async (reference): Promise<string | undefined> => sources[reference],
	});
	expect(
		findings.some(
			(finding): boolean =>
				finding.rule === 'shared-navigation' && finding.message.includes('also own authored'),
		),
	).toBe(true);
});
