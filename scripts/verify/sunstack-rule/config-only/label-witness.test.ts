import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import { test } from 'vitest';

import { labelSpans, neutralizeLabels } from './label-witness.ts';

const source = Buffer.from('<ex-mermaid label="文\r\né" alt="Keep">Tail</ex-mermaid>');
const span = { start: 12, end: 27, text: 'label="文\r\né"' };
test('whole UTF8 attribute is masked while byte positions, CRLF and standard attributes remain exact', () => {
	const output = neutralizeLabels(source, [span]);
	assert.equal(output.length, source.length);
	assert.equal(
		output.toString('utf8'),
		'<ex-mermaid           \r\n    alt="Keep">Tail</ex-mermaid>',
	);
});
test('rejects value-only, shifted and overlapping parser spans', () => {
	assert.throws(() => neutralizeLabels(source, [{ start: 19, end: 22, text: '文' }]));
	assert.throws(() => neutralizeLabels(source, [{ ...span, start: 11 }]));
	assert.throws(() => neutralizeLabels(source, [span, span]));
});
test('parser listing must carry the original document identity', () => {
	const record = {
		where: {
			file: 'page.html',
			span: {
				byte_start: span.start,
				byte_end: span.end,
				source: { content_identity: createHash('sha256').update(source).digest('hex') },
			},
		},
		text: span.text,
	};
	assert.deepEqual(labelSpans({ pieces: [record, record] }, 'page.html', source), [span]);
	assert.throws(() => labelSpans({ pieces: [record] }, 'page.html', Buffer.from('different')));
});
