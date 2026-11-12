import assert from 'node:assert/strict';
import { test } from 'node:test';

import { parseExactJson, widenNamed } from '../src/reply-decoding.ts';
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
