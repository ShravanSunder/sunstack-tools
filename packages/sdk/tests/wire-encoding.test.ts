import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { WideShape } from '../src/contracts/wide-integer-shapes.ts';
import {
	decodeDeserializationReply,
	isInspectionOutcome,
	parseExactJson,
	widenNamed,
} from '../src/reply-decoding.ts';
import { encodeForNative } from '../src/wire-encoding.ts';

function rejected(value: unknown): string {
	const encoded = encodeForNative(value);
	assert.equal(encoded.kind, 'rejected', `expected a refusal for ${String(value)}`);
	return encoded.kind === 'rejected' ? encoded.message : '';
}

await test('bigint crosses as an exact decimal token and absent optionals stay absent', () => {
	const encoded = encodeForNative({
		limits: { max_file_bytes: 18446744073709551615n },
		label: undefined,
	});
	assert.deepEqual(encoded, {
		kind: 'encoded',
		json: '{"limits":{"max_file_bytes":18446744073709551615}}',
	});
});

await test('non-data values are refused instead of disappearing into JSON', () => {
	assert.match(rejected({ limit: Number.MAX_SAFE_INTEGER + 2 }), /unsafe integer/u);
	assert.match(rejected({ limit: 2n ** 64n }), /64-bit integer range/u);
	assert.match(rejected({ limit: -(2n ** 63n) - 1n }), /64-bit integer range/u);
	assert.equal(encodeForNative({ limit: -(2n ** 63n) }).kind, 'encoded');
	assert.match(rejected({ limit: Number.NaN }), /finite/u);
	assert.match(rejected({ limit: Number.POSITIVE_INFINITY }), /finite/u);
	assert.match(rejected({ hook: (): void => {} }), /function/u);
	assert.match(rejected({ tag: Symbol('tag') }), /symbol/u);
	assert.match(rejected({ when: new Date(0) }), /plain object/u);
	assert.match(rejected({ pending: Promise.resolve(1) }), /plain object/u);
	assert.match(rejected({ items: [1, undefined] }), /undefined/u);
	const cycle: Record<string, unknown> = {};
	cycle['self'] = cycle;
	assert.match(rejected(cycle), /cycle/u);
});

await test('holes, symbol keys and inspection failures are refused; special keys stay data', () => {
	const sparse: unknown[] = [1];
	sparse[2] = 3;
	assert.match(rejected({ items: sparse }), /sparse/u);
	assert.equal(encodeForNative({ items: [1, 2, 3] }).kind, 'encoded');

	assert.match(rejected({ [Symbol('hidden')]: 1, visible: 1 }), /symbol-keyed/u);
	assert.equal(encodeForNative({ visible: 1 }).kind, 'encoded');

	const throwing = Object.defineProperty({}, 'value', {
		enumerable: true,
		get(): never {
			throw new Error('getter failed');
		},
	});
	assert.match(rejected(throwing), /could not be inspected/u);
	const revoked = Proxy.revocable({}, {});
	revoked.revoke();
	assert.match(rejected({ nested: revoked.proxy }), /could not be inspected/u);

	// Keys strict Rust must judge are carried, never dropped or turned into prototypes.
	const special: unknown = JSON.parse('{"__proto__":{"polluted":true},"constructor":1}');
	const encoded = encodeForNative(special);
	assert.deepEqual(encoded, {
		kind: 'encoded',
		json: '{"__proto__":{"polluted":true},"constructor":1}',
	});
	const hidden = Object.defineProperty({ shown: 1 }, 'unlisted', { value: 2, enumerable: false });
	assert.deepEqual(encodeForNative(hidden), { kind: 'encoded', json: '{"shown":1,"unlisted":2}' });
});

await test('reply integers beyond the safe range decode exactly', () => {
	const parsed = parseExactJson('{"a":18446744073709551615,"b":2,"c":1.5,"d":1e21}');
	assert.deepEqual(parsed, { a: 18446744073709551615n, b: 2, c: 1.5, d: 1e21 });
});

await test('full-width contract fields decode as bigint even when small', () => {
	const reply = parseExactJson(
		'{"kind":"valid","summary":{"rulebook_count":1,"active_rule_count":1,"skipped_rule_count":0,"evidence_kind_count":1,"filter_count":0},"sources":[]}',
	);
	const widened = widenNamed(reply, 'CheckOutcome');
	// u32 counts stay numbers.
	assert.deepEqual(widened, {
		kind: 'valid',
		summary: {
			rulebook_count: 1,
			active_rule_count: 1,
			skipped_rule_count: 0,
			evidence_kind_count: 1,
			filter_count: 0,
		},
		sources: [],
	});
});

await test('document decoding widens only the selected root shape', () => {
	// The same path is u64 in a report, but is not a configuration limit.
	// These short envelopes exercise trusted-native dispatch, not Rust validation.
	const configuration = decodeDeserializationReply(
		'{"kind":"decoded","value":{"kind":"configuration","value":{"limits":{"max_file_bytes":4,"max_request_bytes":7}}}}',
	);
	const report = decodeDeserializationReply(
		'{"kind":"decoded","value":{"kind":"report","value":{"limits":{"max_file_bytes":4,"max_request_bytes":7}}}}',
	);
	assert.deepEqual(configuration, {
		kind: 'decoded',
		value: {
			kind: 'configuration',
			value: { limits: { max_file_bytes: 4n, max_request_bytes: 7 } },
		},
	});
	assert.deepEqual(report, {
		kind: 'decoded',
		value: { kind: 'report', value: { limits: { max_file_bytes: 4n, max_request_bytes: 7n } } },
	});
	assert.equal(
		decodeDeserializationReply('{"kind":"decoded","value":{"kind":"unknown","value":{}}}'),
		undefined,
	);
	assert.equal(decodeDeserializationReply('{'), undefined);
});

await test('same-path wide and narrow private roots reject cross-root widening', () => {
	// Synthetic metadata, not SDK schemas: both payloads own `shared.count`.
	// One declares u64/bigint, the other u32/number (no widening descriptor).
	const wideShape = {
		object: { shared: { object: { count: 'bigint' } } },
	} as const satisfies WideShape;
	const fixture = {
		roots: { configuration: wideShape, report: null },
		definitions: {},
	};
	const wideText =
		'{"kind":"decoded","value":{"kind":"configuration","value":{"shared":{"count":7}}}}';
	const narrowText = '{"kind":"decoded","value":{"kind":"report","value":{"shared":{"count":7}}}}';
	const expectedWide = {
		kind: 'decoded',
		value: { kind: 'configuration', value: { shared: { count: 7n } } },
	};
	const expectedNarrow = {
		kind: 'decoded',
		value: { kind: 'report', value: { shared: { count: 7 } } },
	};
	assert.deepEqual(decodeDeserializationReply(wideText, fixture), expectedWide);
	assert.deepEqual(decodeDeserializationReply(narrowText, fixture), expectedNarrow);

	// Counterfactual: a union walker uses the wide root on this narrow payload.
	// The unchanged narrow oracle detects it, while the wide control still passes.
	const wrongFixture = {
		...fixture,
		roots: { ...fixture.roots, report: { union: [wideShape] } },
	};
	const wrongNarrow = decodeDeserializationReply(narrowText, wrongFixture);
	assert.notDeepEqual(wrongNarrow, expectedNarrow);
	assert.deepEqual(wrongNarrow, {
		kind: 'decoded',
		value: { kind: 'report', value: { shared: { count: 7n } } },
	});
	assert.deepEqual(decodeDeserializationReply(wideText, wrongFixture), expectedWide);
});

await test('inspection reply envelopes retain their closed status variants', () => {
	assert.equal(
		isInspectionOutcome({ kind: 'inspected', report: { status: { kind: 'clean' } } }),
		true,
	);
	assert.equal(
		isInspectionOutcome({ kind: 'inspected', report: { status: { kind: 'findings' } } }),
		false,
	);
	assert.equal(
		isInspectionOutcome({ kind: 'incomplete', report: { status: { kind: 'incomplete' } } }),
		true,
	);
	assert.equal(isInspectionOutcome({ kind: 'cancelled', report: null }), true);
	assert.equal(isInspectionOutcome({ kind: 'invalid', diagnostics: [] }), true);
	assert.equal(isInspectionOutcome({ kind: 'failed', failure: {}, report: null }), true);
});
