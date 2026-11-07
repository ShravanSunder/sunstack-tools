import { expect, test } from 'vitest';

import { classifyDocumentLink } from './document-links.ts';

test.each([
	['#story', 'local', 'story'],
	['page.html?view=work#story', 'local', 'story'],
	['https://example.test/docs/page.html?view=work#story', 'local', 'story'],
	['#entry%20one', 'local', 'entry one'],
	['other.html?view=work#story', 'other', undefined],
	['page.html?view=other#story', 'other', undefined],
	['https://other.test/docs/page.html?view=work#story', 'other', undefined],
	['#', 'other', undefined],
	['#%zz', 'invalid', undefined],
	['http://[invalid', 'invalid', undefined],
])('classifies the actual destination of %s', (href, expectedKind, expectedReference): void => {
	// Arrange: literal contract cases distinguish document identity from a fragment's appearance.
	const documentUrl = new URL('https://example.test/docs/page.html?view=work');
	// Act.
	const result = classifyDocumentLink(href, documentUrl);
	// Assert: a different allowed spelling retains the same local identity.
	expect(result.kind).toBe(expectedKind);
	if (result.kind === 'local') expect(result.reference).toBe(expectedReference);
});

test('a different base resolves links without changing the document identity', (): void => {
	const result = classifyDocumentLink(
		'page.html#story',
		new URL('https://example.test/docs/page.html'),
		new URL('https://example.test/other/'),
	);
	expect(result.kind).toBe('other');
});
