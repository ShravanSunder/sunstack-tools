import assert from 'node:assert/strict';

import { test } from 'vitest';

import { requireLegacyParity } from './parity.ts';
test('legacy comparison permits only the root version, not hashes or nested versions', () => {
	const before = Buffer.from(
		JSON.stringify(
			{ version: '0.15.0', request: { hash: 'original', version: 'nested' } },
			null,
			2,
		),
	);
	const after = Buffer.from(
		JSON.stringify(
			{ version: '0.15.1', request: { hash: 'original', version: 'nested' } },
			null,
			2,
		),
	);
	requireLegacyParity(before, after);
	assert.throws(() =>
		requireLegacyParity(before, Buffer.from(after.toString().replace('original', 'changed'))),
	);
	assert.throws(() =>
		requireLegacyParity(before, Buffer.from(after.toString().replace('nested', 'changed'))),
	);
});
