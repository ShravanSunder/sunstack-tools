/** Installed native SDK selection using the same independently authored fixture. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { isDeepStrictEqual } from 'node:util';

import { compactJson, jsonSlice, requirePayloadBytes } from '../prepare/raw-json.ts';
import { parseObject, textValue } from '../rulebook/rulebook-fixture-support.ts';
import { boundedSdkWait } from '../sdk/sdk-controlled-backend.ts';
import {
	requireCompletedSdkReport,
	requireSdkKind,
	type JourneyContext,
} from '../sdk/sdk-journeys.ts';
import { requireSdk, saveSdkEvidence } from '../sdk/sdk-target.ts';
import {
	attributeControlledCaptures,
	requireCaseAccounting,
	requireControlledDispatch,
	requestOwnerAt,
} from './case-dispatch-checks.ts';
import { createCaseDispatchFixture, selectedText } from './case-dispatch-fixture.ts';

function objectValue(value: unknown): Record<string, unknown> {
	assert.ok(value !== null && typeof value === 'object' && !Array.isArray(value));
	return Object.fromEntries(Object.entries(value));
}

function arrayValue(value: unknown): unknown[] {
	assert.ok(Array.isArray(value));
	return Array.from(value);
}

function requestDigest(raw: string, index: number): string {
	return createHash('sha256')
		.update(compactJson(jsonSlice(raw, ['requests', index, 'request'])))
		.digest('hex');
}

/** Caller owns the pinned installed SDK, started controlled backend and evidence directory. */
export async function observeInstalledCaseDispatch(context: JourneyContext): Promise<void> {
	const root = path.join(context.evidence, 'case-dispatch-sdk-fixture');
	createCaseDispatchFixture(root, context.backend.endpoint);
	const sdk = context.sdk;
	requireSdk(sdk.serialize && sdk.deserialize, 'Installed SDK codec is unavailable');
	const serialize = sdk.serialize;
	const deserialize = sdk.deserialize;
	const loaded = requireSdkKind(
		await boundedSdkWait(sdk.load({ directory: root }), 'case SDK load'),
		'loaded',
	);
	const binding = {
		id: 'selected-a',
		kind: 'rule',
		rule: 'approval',
		file: 'docs/pieces.md',
		lines: [1n, 1n],
		text_sha256: createHash('sha256').update(selectedText).digest('hex'),
	};
	const options = { root, scope: { kind: 'all' } };
	const run = async (
		selection: readonly object[] | undefined,
	): Promise<{ outcome: Record<string, unknown>; raw: string; captures: string[] }> => {
		const before = context.backend.captures.length;
		const cancellation = new AbortController();
		const pending = sdk.run(loaded['value'], {
			...options,
			...(selection === undefined ? {} : { case_selection: selection }),
			signal: cancellation.signal,
		});
		let outcome: Record<string, unknown>;
		try {
			outcome = requireSdkKind(
				await boundedSdkWait(pending, 'installed case selection'),
				'completed',
			);
		} finally {
			cancellation.abort();
			await boundedSdkWait(Promise.allSettled([pending]), 'installed case run settlement');
		}
		requireCompletedSdkReport(outcome);
		const encoded = objectValue(serialize({ kind: 'report', value: outcome['report'] }));
		requireSdk(
			encoded['kind'] === 'serialized' && typeof encoded['json'] === 'string',
			'Actual SDK report did not serialize',
		);
		const raw = jsonSlice(encoded['json'], ['payload', 'value']);
		return {
			outcome,
			raw,
			captures: context.backend.captures.slice(before).map((capture) => capture.raw),
		};
	};
	const ordinary = await run(undefined);
	assert.equal(objectValue(ordinary.outcome['report'])['case_selection'], undefined);
	const chosen = await run([
		binding,
		{ ...binding, id: 'selected-b' },
		{ ...binding, id: 'stale', text_sha256: '0'.repeat(64) },
		{ ...binding, id: 'unmatched', rule: 'absent' },
	]);
	const report = objectValue(chosen.outcome['report']);
	const accounting = objectValue(report['case_selection']);
	requireCaseAccounting({
		selectedIds: ['selected-a', 'selected-b', 'stale', 'unmatched'],
		dispositions: arrayValue(accounting['bindings'])
			.map(objectValue)
			.map((entry) => ({
				id: textValue(objectValue(entry['binding'])['id']),
				kind: textValue(objectValue(entry['outcome'])['kind']),
			})),
		expectedKinds: new Map([
			['selected-a', 'selected'],
			['selected-b', 'selected'],
			['stale', 'stale'],
			['unmatched', 'unmatched'],
		]),
	});
	const excludedBindings = arrayValue(accounting['excluded']).map(objectValue);
	assert.ok(
		excludedBindings.some(
			(owner) =>
				owner['kind'] === 'rule' &&
				owner['rule'] === 'approval' &&
				owner['file'] === 'docs/pieces.md' &&
				isDeepStrictEqual(owner['lines'], [3n, 3n]),
		),
		'SDK selection omitted the excluded owner',
	);
	for (const entry of arrayValue(accounting['bindings']).map(objectValue)) {
		const id = objectValue(entry['binding'])['id'];
		if (id === 'selected-a' || id === 'selected-b') {
			assert.deepEqual(
				objectValue(entry['outcome'])['owner'],
				{ kind: 'rule', rule: 'approval', file: 'docs/pieces.md', lines: [1n, 1n] },
				'Selected binding points at another execution owner',
			);
		}
	}
	const selectedOwner = JSON.stringify(['rule', 'approval', 'docs/pieces.md', '1', '1']);
	const excludedOwner = JSON.stringify(['rule', 'approval', 'docs/pieces.md', '3', '3']);
	requireControlledDispatch({
		ordinary: attributeControlledCaptures(ordinary.raw, ordinary.captures),
		selected: attributeControlledCaptures(chosen.raw, chosen.captures),
		selectedOwners: [selectedOwner],
		excludedOwners: [excludedOwner],
		prerequisiteOwners: [],
	});
	const ordinaryRequests = arrayValue(parseObject(ordinary.raw)['requests']);
	const selectedRequests = arrayValue(parseObject(chosen.raw)['requests']);
	const ordinaryIndex = ordinaryRequests.findIndex(
		(_row, index) => requestOwnerAt(ordinary.raw, index) === selectedOwner,
	);
	const selectedIndex = selectedRequests.findIndex(
		(_row, index) => requestOwnerAt(chosen.raw, index) === selectedOwner,
	);
	assert.ok(
		ordinaryIndex >= 0 && selectedIndex >= 0 && selectedRequests.length === 1,
		'Selected SDK request inventory differs',
	);
	requirePayloadBytes({
		left: ordinary.raw,
		leftPath: ['requests', ordinaryIndex, 'request'],
		right: chosen.raw,
		rightPath: ['requests', selectedIndex, 'request'],
		label: 'Selected SDK request payload differs',
	});
	const ordinaryDigest = requestDigest(ordinary.raw, ordinaryIndex);
	const selectedDigest = requestDigest(chosen.raw, selectedIndex);
	assert.equal(selectedDigest, ordinaryDigest, 'Independently computed SDK input digest differs');
	const empty = await run([]);
	assert.equal(empty.captures.length, 0, 'Explicit empty selection dispatched HTTP');
	assert.deepEqual(
		arrayValue(objectValue(objectValue(empty.outcome['report'])['case_selection'])['bindings']),
		[],
	);
	const wideInput = { ...binding, id: 'wide', lines: [9007199254740993n, 9007199254740994n] };
	const wideRun = await run([wideInput]);
	assert.equal(wideRun.captures.length, 0, 'Out-of-range owner unexpectedly dispatched');
	const wideBinding = objectValue(
		arrayValue(
			objectValue(objectValue(wideRun.outcome['report'])['case_selection'])['bindings'],
		)[0],
	);
	assert.deepEqual(objectValue(wideBinding['binding'])['lines'], wideInput.lines);
	assert.equal(objectValue(wideBinding['outcome'])['kind'], 'stale');
	const encoded = objectValue(serialize({ kind: 'report', value: report }));
	requireSdk(
		encoded['kind'] === 'serialized' && typeof encoded['json'] === 'string',
		'Selected report could not be serialized',
	);
	const decoded = objectValue(
		await boundedSdkWait(deserialize(encoded['json']), 'selected report decode'),
	);
	assert.equal(decoded['kind'], 'decoded');
	assert.ok(
		isDeepStrictEqual(decoded['value'], { kind: 'report', value: report }),
		'Selected report changed in roundtrip',
	);
	const wide = encoded['json'].replace(
		'"lines":[1,1]',
		'"lines":[9007199254740993,9007199254740994]',
	);
	assert.notEqual(wide, encoded['json'], 'No binding line integers in saved report');
	const wideDecoded = objectValue(await boundedSdkWait(deserialize(wide), 'wide report decode'));
	assert.equal(wideDecoded['kind'], 'decoded');
	const wideReport = objectValue(objectValue(wideDecoded['value'])['value']);
	const first = objectValue(arrayValue(objectValue(wideReport['case_selection'])['bindings'])[0]);
	assert.deepEqual(objectValue(first['binding'])['lines'], [9007199254740993n, 9007199254740994n]);
	const overflow = wide.replace('9007199254740993', '18446744073709551616');
	assert.notEqual(overflow, wide);
	assert.equal(
		objectValue(await boundedSdkWait(deserialize(overflow), 'overflow report decode'))['kind'],
		'invalid',
	);
	const revision = parseObject(encoded['json'])['contract_revision'];
	assert.equal(typeof revision, 'number');
	if (typeof revision !== 'number') throw new Error('Missing contract revision');
	const oldRevision = encoded['json'].replace(
		`"contract_revision":${revision}`,
		`"contract_revision":${revision - 1}`,
	);
	assert.notEqual(oldRevision, encoded['json'], 'No contract revision in saved report');
	assert.equal(
		objectValue(await boundedSdkWait(deserialize(oldRevision), 'revision mismatch'))['kind'],
		'incompatible',
	);
	saveSdkEvidence(path.join(context.evidence, 'case-dispatch-sdk.json'), {
		ordinary: ordinary.outcome,
		selected: chosen.outcome,
		empty: empty.outcome,
		wideInput: wideRun.outcome,
		ordinaryHttp: ordinary.captures,
		selectedHttp: chosen.captures,
		selectedEncoded: encoded['json'],
		ordinaryInputDigest: ordinaryDigest,
		selectedInputDigest: selectedDigest,
		wideDecoded,
	});
}
