/** C5: asking/budget results come from the released witness; raw data comes from originals. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import { arrayValue, numberValue, objectValue } from '../rulebook/rulebook-fixture-support.ts';

export function requireDefaultProjection(
	originalBytes: Buffer,
	witnessBytes: Buffer,
	candidateBytes: Buffer,
): void {
	const expected = objectValue(JSON.parse(originalBytes.toString('utf8')));
	const witness = objectValue(JSON.parse(witnessBytes.toString('utf8')));
	const actual = objectValue(JSON.parse(candidateBytes.toString('utf8')));
	assert.equal(expected['version'], '0.14.1');
	assert.equal(witness['version'], '0.14.1');
	assert.equal(actual['version'], '0.15.0');
	const suffix = originalBytes.toString('utf8').endsWith('\n') ? '\n' : '';
	assert.equal(JSON.stringify(expected, null, 2) + suffix, originalBytes.toString('utf8'));
	const originalPieces = objectValue(expected['piece_evidence']);
	const witnessPieces = objectValue(witness['piece_evidence']);
	assert.deepEqual(
		Object.keys(originalPieces),
		Object.keys(witnessPieces),
		'Witness changed owners',
	);
	for (const [name, value] of Object.entries(originalPieces)) {
		const piece = objectValue(objectValue(value)['piece']);
		const counterpart = objectValue(objectValue(witnessPieces[name])['piece']);
		const where = objectValue(piece['where']);
		const target = objectValue(counterpart['where']);
		assert.equal(where['file'], target['file']);
		for (const key of ['byte_start', 'byte_end'])
			assert.equal(objectValue(where['span'])[key], objectValue(target['span'])[key]);
		piece['model_view'] = structuredClone(objectValue(counterpart['model_view']));
	}
	const requests = arrayValue(expected['requests']).map(objectValue);
	const witnessRequests = arrayValue(witness['requests']).map(objectValue);
	assert.equal(requests.length, witnessRequests.length);
	for (const record of requests) {
		const match = witnessRequests.find(
			(value) => numberValue(value['id']) === numberValue(record['id']),
		);
		assert.ok(match);
		assert.equal(objectValue(record['owner'])['rule'], objectValue(match['owner'])['rule']);
		record['request'] = structuredClone(objectValue(match['request']));
		const hash = createHash('sha256').update(JSON.stringify(record['request'])).digest('hex');
		assert.equal(match['input_sha256'], hash, 'Witness hash is not derived from its request');
		record['input_sha256'] = hash;
	}
	expected['version'] = '0.15.0';
	assert.equal(
		JSON.stringify(expected, null, 2) + suffix,
		candidateBytes.toString('utf8'),
		'Default report changed beyond witnessed asking projections/hashes',
	);
}
