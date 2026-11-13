/** Exact additive wire boundary; the old bounded view, never marker inference, owns cuts. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import {
	arrayValue,
	objectValue,
	textValue,
	type JsonObject,
	type JsonValue,
} from '../rulebook/rulebook-fixture-support.ts';

function containsMarker(value: JsonValue): boolean {
	if (typeof value === 'string')
		return /\[cut(?:\]|: (?:\d+ more|html_projection_unrecoverable_markup)\])/u.test(value);
	if (Array.isArray(value)) return value.some(containsMarker);
	return value !== null && typeof value === 'object' && Object.values(value).some(containsMarker);
}
function removeAddedCuts(fields: JsonObject, cut: string[]): void {
	assert.deepEqual(
		fields['cut-fields'],
		cut.length > 0 ? cut : undefined,
		'Wrong cut-field vector or empty omission',
	);
	delete fields['cut-fields'];
}
function validatedViewCuts(
	input: JsonObject,
	view: JsonObject,
	markerEquivalence: boolean,
): string[] {
	assert.equal(input['cut-fields'], undefined, 'Old selection state already contains cut metadata');
	assert.equal(view['state'], 'bounded');
	assert.deepEqual(input, view['fields'], 'Old input does not match its offered view');
	const cut = arrayValue(view['cut']).map(textValue);
	if (markerEquivalence) {
		const marked = Object.entries(input)
			.filter(([, value]) => containsMarker(value))
			.map(([name]) => name);
		assert.deepEqual(cut, marked, 'Generated marker fields differ from the bounded cut vector');
	}
	return cut;
}
export function requireCutParity(options: {
	before: Buffer;
	after: Buffer;
	markerEquivalence: boolean;
}): JsonObject {
	const expected = objectValue(JSON.parse(options.before.toString('utf8')));
	const actual = objectValue(JSON.parse(options.after.toString('utf8')));
	assert.equal(expected['version'], '0.15.1');
	assert.equal(actual['version'], '0.16.0');
	const suffix = options.before.toString('utf8').endsWith('\n') ? '\n' : '';
	assert.ok(
		Buffer.from(JSON.stringify(expected, null, 2) + suffix).equals(options.before),
		'Baseline is not canonical JSON',
	);
	assert.ok(
		Buffer.from(JSON.stringify(actual, null, 2) + suffix).equals(options.after),
		'Candidate changed report formatting',
	);
	let candidateCut = 0;
	let candidateUncut = 0;
	let sharedCut = 0;
	let sharedUncut = 0;
	let changedHashes = 0;
	let unchangedHashes = 0;
	const actualRequests = arrayValue(actual['requests']);
	for (const [index, value] of arrayValue(expected['requests']).entries()) {
		const record = objectValue(value);
		if (objectValue(record['owner'])['kind'] !== 'selection') continue;
		const request = objectValue(record['request']);
		const oldHash = createHash('sha256').update(JSON.stringify(request)).digest('hex');
		assert.equal(record['input_sha256'], oldHash);
		const actualRecord = objectValue(actualRequests[index]);
		const actualRequest = objectValue(actualRecord['request']);
		const hash = createHash('sha256').update(JSON.stringify(actualRequest)).digest('hex');
		assert.equal(
			actualRecord['input_sha256'],
			hash,
			'Candidate hash is not derived from its complete request',
		);
		const actualState = objectValue(actualRequest['state']);
		const actualInputs = objectValue(actualState['candidates']);
		const state = objectValue(request['state']);
		const inputs = objectValue(state['candidates']);
		const evidence = objectValue(record['selection_evidence']);
		const descriptors = arrayValue(evidence['candidates']).map(objectValue);
		const keys = descriptors.map((candidate) => textValue(candidate['key']));
		assert.equal(new Set(keys).size, keys.length);
		assert.deepEqual(Object.keys(inputs).toSorted(), keys.toSorted());
		assert.deepEqual(Object.keys(actualInputs).toSorted(), keys.toSorted());
		for (const candidate of descriptors) {
			const key = textValue(candidate['key']);
			const fields = objectValue(inputs[key]);
			const cut = validatedViewCuts(
				fields,
				objectValue(candidate['selection_view']),
				options.markerEquivalence,
			);
			removeAddedCuts(objectValue(actualInputs[key]), cut);
			if (cut.length > 0) candidateCut++;
			else candidateUncut++;
		}
		const shared = objectValue(evidence['shared']);
		const fields = objectValue(state['shared']);
		const cut = validatedViewCuts(fields, objectValue(shared['view']), options.markerEquivalence);
		assert.deepEqual(state['where'], { file: shared['file'] ?? null });
		assert.deepEqual(Object.keys(state).toSorted(), ['candidates', 'shared', 'where']);
		removeAddedCuts(objectValue(actualState['shared']), cut);
		if (cut.length > 0) sharedCut++;
		else sharedUncut++;
		actualRecord['input_sha256'] = oldHash;
		if (hash === oldHash) unchangedHashes++;
		else changedHashes++;
	}
	actual['version'] = '0.15.1';
	assert.ok(
		Buffer.from(JSON.stringify(actual, null, 2) + suffix).equals(options.before),
		'Report changed outside exact cut-fields/root-version/computed-hash boundary; inspect retained reports',
	);
	return {
		candidate_cut: candidateCut,
		candidate_uncut: candidateUncut,
		shared_cut: sharedCut,
		shared_uncut: sharedUncut,
		changed_hashes: changedHashes,
		unchanged_hashes: unchangedHashes,
	};
}
