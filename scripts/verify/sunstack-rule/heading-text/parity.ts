import assert from 'node:assert/strict';

import { objectValue } from '../rulebook/rulebook-fixture-support.ts';

/** Full report byte equality, allowing exactly its root version field. */
export function requireLegacyParity(before: Buffer, after: Buffer): void {
	const original = objectValue(JSON.parse(before.toString('utf8')));
	const candidate = objectValue(JSON.parse(after.toString('utf8')));
	assert.equal(original['version'], '0.15.0');
	assert.equal(candidate['version'], '0.15.1');
	const suffix = before.toString('utf8').endsWith('\n') ? '\n' : '';
	assert.ok(
		Buffer.from(JSON.stringify(original, null, 2) + suffix).equals(before),
		'Noncanonical baseline report',
	);
	original['version'] = '0.15.1';
	assert.ok(
		Buffer.from(JSON.stringify(original, null, 2) + suffix).equals(after),
		'Legacy report changed beyond root version; inspect both retained reports',
	);
}
