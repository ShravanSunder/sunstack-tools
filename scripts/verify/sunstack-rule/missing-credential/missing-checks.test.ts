import { describe, expect, it } from 'vitest';

import {
	integer,
	requireCaseLinks,
	requireLocalParity,
	requireRequestParity,
	requireProviderFailure,
	requireUnavailableCounts,
	requireUnavailableResults,
	requireUnavailableSelection,
	requireZeroDispatch,
	stable,
} from './missing-checks.ts';

function fixture(): {
	results: { id: number; name: string; outcome: string; text_sha256: string; payload: string }[];
	findings: { id: number; rule: string; message: string }[];
} {
	return {
		results: ['first', 'second'].map((name, index) => ({
			id: index + 1,
			name,
			outcome: 'finding',
			text_sha256: 'a'.repeat(64),
			payload: `original ${name}`,
		})),
		findings: ['first', 'second'].map((rule, index) => ({ id: index + 1, rule, message: rule })),
	};
}
function parity(mixed: unknown): void {
	requireLocalParity({
		mechanical: fixture(),
		mixed,
		names: ['first', 'second'],
		expectedFindings: 2,
		expectedSha256: 'a'.repeat(64),
	});
}
describe('independent local facts', () => {
	it('accepts result ordering and run-local IDs without discarding facts', () => {
		const mixed = fixture();
		mixed.results.reverse();
		mixed.findings.reverse();
		for (const row of mixed.results) row.id += 20;
		for (const row of mixed.findings) row.id += 20;
		expect(() => parity(mixed)).not.toThrow();
	});
	it.each(['drop', 'duplicate', 'content', 'hash', 'finding'] as const)('rejects %s', (fault) => {
		const mixed = fixture();
		const first = mixed.results[0];
		const finding = mixed.findings[0];
		if (!first || !finding) throw new Error('Broken fixture');
		switch (fault) {
			case 'drop':
				mixed.results.pop();
				break;
			case 'duplicate':
				mixed.results.push({ ...first });
				break;
			case 'content':
				first.payload = 'changed';
				break;
			case 'hash':
				first.text_sha256 = 'b'.repeat(64);
				break;
			case 'finding':
				finding.message = 'changed';
				break;
		}
		expect(() => parity(mixed)).toThrow();
	});
});
function unavailable(): Record<string, unknown> {
	return {
		results: [
			{
				id: 4n,
				name: 'judge',
				outcome: 'unavailable',
				text_sha256: 'a'.repeat(64),
				reason: { kind: 'credential_unavailable' },
				request_ids: [],
				answers: {},
			},
		],
		filter_answers: [],
		selections: [],
		incomplete: [],
		unavailable_counts: {
			rule_pieces: 1n,
			filter_folders: 0n,
			selection_candidates: 0n,
			evidence_scopes: 0n,
		},
		requests: [],
		usage: {
			model_requests: 0n,
			input_tokens: null,
			output_tokens: null,
			cost_usd: null,
			cost_reported_requests: 0,
			cost_missing_requests: 0,
		},
	};
}
it('accepts exact unavailable facts and their counts', () => {
	const value = unavailable();
	requireUnavailableResults(value, new Map([['judge', 'credential_unavailable']]));
	requireUnavailableCounts(value);
	requireZeroDispatch(value, []);
});
it('rejects absent reasons and wrong reasons', () => {
	for (const reason of [undefined, { kind: 'filter_unavailable' }]) {
		const value = unavailable();
		value['results'] = [
			{ id: 4n, name: 'judge', outcome: 'unavailable', text_sha256: 'a'.repeat(64), reason },
		];
		expect(() =>
			requireUnavailableResults(value, new Map([['judge', 'credential_unavailable']])),
		).toThrow();
	}
});
it('rejects forged zero unavailable count', () => {
	const value = unavailable();
	value['unavailable_counts'] = {
		rule_pieces: 0n,
		filter_folders: 0n,
		selection_candidates: 0n,
		evidence_scopes: 0n,
	};
	expect(() => requireUnavailableCounts(value)).toThrow('Forged rule_pieces');
});
it('rejects nonzero HTTP and fabricated request accounting independently', () => {
	expect(() => requireZeroDispatch(unavailable(), ['POST'])).toThrow('HTTP');
	const value = unavailable();
	value['usage'] = { model_requests: 1n, input_tokens: 0n, output_tokens: 0n };
	expect(() => requireZeroDispatch(value, [])).toThrow('model requests');
});
it('keeps selected multiplicity and real links', () => {
	const expected = new Map([
		['known', { kind: 'unavailable', linked: true }],
		['unknown', { kind: 'unavailable', linked: false }],
	]);
	const cases = [
		{ id: 'known', outcome: { kind: 'unavailable' }, result_id: 9n },
		{ id: 'unknown', outcome: { kind: 'unavailable' }, result_id: null },
	];
	requireCaseLinks({ cases, expected, actualResultIds: [9n] });
	expect(() =>
		requireCaseLinks({ cases: [...cases, cases[0]], expected, actualResultIds: [9n] }),
	).toThrow('denominator');
	expect(() => requireCaseLinks({ cases, expected, actualResultIds: [8n] })).toThrow(
		'invented result',
	);
});
it('requires nonempty full request bytes and preserves exact integer/string distinction', () => {
	requireRequestParity(['{"model":"m","state":{}}'], ['{"model":"m","state":{}}']);
	expect(() => requireRequestParity([], [])).toThrow('witness');
	expect(() => requireRequestParity(['{"model":"m","state":{}}'], ['{"state":{}}'])).toThrow(
		'bytes',
	);
	expect(stable(18446744073709551615n)).not.toBe(stable('18446744073709551615'));
	expect(() => integer(9007199254740992)).toThrow('exact integer');
});

it('preserves real HTTP failure class and nonzero actual attempts', () => {
	const value = {
		requests: [
			{
				state: 'run',
				outcome: {
					kind: 'provider_failure',
					failure_class: 'authentication',
					http_status: 401,
					attempts: [
						{ ordinal: 1, evidence: { kind: 'remote_outcome_unknown', http_status: 401 } },
					],
				},
			},
		],
	};
	requireProviderFailure(value, 401, 1);
	expect(() => requireProviderFailure(value, 400, 1)).toThrow();
	expect(() => requireProviderFailure(value, 401, 0)).toThrow('witness');
	expect(() => requireProviderFailure(value, 401, 2)).toThrow('attempts');
});

it('does not turn unavailable selection into negative selection or conclusive absence', () => {
	const selection = {
		candidates: 1n,
		unavailable: 1n,
		asked: 0n,
		selected: 0n,
		not_selected: 0n,
		uncertain: 0n,
		incomplete: 0n,
	};
	const value = {
		selections: [{ outcome: { kind: 'unavailable', reason: { kind: 'credential_unavailable' } } }],
		file_counts: [{ kinds: { selected: { selection } } }],
		incomplete: [
			{ reason: { kind: 'selection_unavailable', evidence_kind: 'child', file: 'docs/one.md' } },
		],
	};
	requireUnavailableSelection(value);
	selection.not_selected = 1n;
	expect(() => requireUnavailableSelection(value)).toThrow('not_selected');
	selection.not_selected = 0n;
	value.incomplete.push(
		value.incomplete[0] ?? {
			reason: { kind: 'selection_unavailable', evidence_kind: 'child', file: 'docs/one.md' },
		},
	);
	expect(() => requireUnavailableSelection(value)).toThrow('Duplicate kind/file');
});
