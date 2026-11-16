import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import { test } from 'vitest';

import { byteFixtureCases, byteFixtureSource } from './byte-fixture.ts';

test('literal ranges distinguish adjacent UTF-8 nodes and preserve CRLF', () => {
	const source = Buffer.from(byteFixtureSource);
	assert.equal(source.toString('hex'), '3c703e636166c3a9207965733c2f703e3c703e6e6f3c2f703e0d0a');
	assert.deepEqual(
		byteFixtureCases.map((entry) => entry.bytes),
		[
			[0, 16],
			[16, 25],
		],
	);
	for (const entry of byteFixtureCases) {
		const actual = createHash('sha256')
			.update(source.subarray(...entry.bytes))
			.digest('hex');
		assert.equal(actual, entry.text_sha256);
	}
	assert.deepEqual(
		byteFixtureCases.map((entry) => entry.expected),
		['pass', 'warn'],
	);
});
