import { expect, test } from 'vitest';

import { inventoryHtml } from './html-corpus.ts';

test('real-source inventory excludes escaped examples, comments and raw script strings', () => {
	const source =
		'<!-- <p id="comment"> --><script>const x = \'<i id="script">\';</script><pre>&lt;template id="body-U1"&gt;</pre><table><tr id="R1"><td>R1 / U1</td></tr></table><a href="other.html#R1">Go</a>';
	expect(inventoryHtml(source)).toEqual({
		ids: ['R1'],
		fragmentHrefs: ['other.html#R1'],
		rowIds: ['R1'],
		firstCells: ['R1 / U1'],
	});
});
test('inventory retains both row identities and only actual fragment hrefs', () => {
	const source =
		'<tr id="R1"><td>R1</td></tr><tr id="R10"><td>R10</td></tr><p title="id=\'fake\'">Literal</p><a href="plain.html">plain</a><a href="p.html#missing">missing</a>';
	expect(inventoryHtml(source)).toEqual({
		ids: ['R1', 'R10'],
		fragmentHrefs: ['p.html#missing'],
		rowIds: ['R1', 'R10'],
		firstCells: ['R1', 'R10'],
	});
});
test('attribute-looking strings inside another quoted attribute are not identities or links', () => {
	expect(inventoryHtml("<p title=\"sample id='fake' href='other#bad'\">text</p>")).toEqual({
		ids: [],
		fragmentHrefs: [],
		rowIds: [],
		firstCells: [],
	});
});
