import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import { test } from 'vitest';

import { arrayValue, objectValue, type JsonObject } from '../rulebook/rulebook-fixture-support.ts';
import { requireCutParity } from './cut-oracle.ts';

function encoded(value: JsonObject): Buffer {
	return Buffer.from(JSON.stringify(value, null, 2));
}
function requestHash(record: JsonObject): void {
	record['input_sha256'] = createHash('sha256')
		.update(JSON.stringify(record['request']))
		.digest('hex');
}
function pair(cut: boolean): { before: Buffer; after: JsonObject } {
	const body = cut ? 'prefix[cut]tail' : 'authored [cut] stays literal';
	const opening = cut ? ['[cut: 7 more]'] : ['authored [cut: 7 more]'];
	const record = {
		owner: { kind: 'selection' },
		request: {
			questions: {},
			state: {
				candidates: { ['c1']: { body } },
				shared: { opening },
				where: { file: 'source.md' },
			},
		},
		input_sha256: '',
		selection_evidence: {
			candidates: [
				{
					key: 'c1',
					selection_view: { state: 'bounded', fields: { body }, cut: cut ? ['body'] : [] },
				},
			],
			shared: {
				file: 'source.md',
				view: { state: 'bounded', fields: { opening }, cut: cut ? ['opening'] : [] },
			},
		},
	};
	requestHash(record);
	const before = encoded({
		version: '0.15.1',
		requests: [record],
		raw: 'unchanged',
		findings: [],
		results: [],
	});
	const after = objectValue(JSON.parse(before.toString('utf8')));
	after['version'] = '0.16.0';
	const saved = objectValue(arrayValue(after['requests'])[0]);
	if (cut) {
		const state = objectValue(objectValue(saved['request'])['state']);
		objectValue(state['candidates'])['c1'] = { body, 'cut-fields': ['body'] };
		state['shared'] = { 'cut-fields': ['opening'], opening };
	}
	requestHash(saved);
	return { before, after };
}
test('adds the exact candidate/shared wire arrays and recomputes their hash', () => {
	const { before, after } = pair(true);
	assert.deepEqual(requireCutParity({ before, after: encoded(after), markerEquivalence: true }), {
		candidate_cut: 1,
		candidate_uncut: 0,
		shared_cut: 1,
		shared_uncut: 0,
		changed_hashes: 1,
		unchanged_hashes: 0,
	});
});
test('omits empty cut lists and preserves hashes despite authored marker text', () => {
	const { before, after } = pair(false);
	assert.deepEqual(requireCutParity({ before, after: encoded(after), markerEquivalence: false }), {
		candidate_cut: 0,
		candidate_uncut: 1,
		shared_cut: 0,
		shared_uncut: 1,
		changed_hashes: 0,
		unchanged_hashes: 1,
	});
});
test.each([
	'missing-candidate',
	'missing-shared',
	'wrong-name',
	'duplicate',
	'text',
	'extra-state',
	'hash',
	'raw',
])('rejects %s outside the additive wire boundary', (mutation) => {
	const { before, after } = pair(true);
	const record = objectValue(arrayValue(after['requests'])[0]);
	const state = objectValue(objectValue(record['request'])['state']);
	const candidate = objectValue(objectValue(state['candidates'])['c1']);
	if (mutation === 'missing-candidate') delete candidate['cut-fields'];
	else if (mutation === 'missing-shared') delete objectValue(state['shared'])['cut-fields'];
	else if (mutation === 'wrong-name') candidate['cut-fields'] = ['opening'];
	else if (mutation === 'duplicate') candidate['cut-fields'] = ['body', 'body'];
	else if (mutation === 'text') candidate['body'] = 'different text';
	else if (mutation === 'extra-state') objectValue(state['candidates'])['c9'] = { body: 'extra' };
	else if (mutation === 'raw') after['raw'] = 'changed';
	requestHash(record);
	if (mutation === 'hash') record['input_sha256'] = 'arbitrary';
	assert.throws(() => requireCutParity({ before, after: encoded(after), markerEquivalence: true }));
});
test('an empty wire array is not a substitute for omission', () => {
	const { before, after } = pair(false);
	const record = objectValue(arrayValue(after['requests'])[0]);
	objectValue(objectValue(objectValue(record['request'])['state'])['shared'])['cut-fields'] = [];
	requestHash(record);
	assert.throws(() =>
		requireCutParity({ before, after: encoded(after), markerEquivalence: false }),
	);
});
test('the added property may occur after existing fields without moving those fields', () => {
	const { before, after } = pair(true);
	const record = objectValue(arrayValue(after['requests'])[0]);
	objectValue(objectValue(record['request'])['state'])['shared'] = {
		opening: ['[cut: 7 more]'],
		'cut-fields': ['opening'],
	};
	requestHash(record);
	assert.doesNotThrow(() =>
		requireCutParity({ before, after: encoded(after), markerEquivalence: true }),
	);
});
