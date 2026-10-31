import { describe, expect, it } from 'vitest';

import { requireRulebookCompatibility, type CompatibilityCase } from './rulebook-compatibility.ts';
const report: CompatibilityCase = {
	report: {
		results: [
			{
				name: 'rule',
				marker: 'K001',
				file: 'SKILL.md',
				first_line: 1,
				last_line: 3,
				outcome: 'uncertain',
				step: 'verdict',
			},
		],
		findings: [],
		uncertain: [{ step: 'verdict', answers: { question: 0.5 } }],
		incomplete: [],
	},
	exit: 5,
	requests: 1,
};
describe('V23 compatibility sensitivity', () => {
	it('accepts the same complete emitted result and rejects a planted step or outcome', () => {
		expect(() => requireRulebookCompatibility(report, structuredClone(report))).not.toThrow();
		for (const field of ['step', 'outcome']) {
			const changed = structuredClone(report);
			const results = changed.report['results'];
			if (!Array.isArray(results)) throw new Error('Fixture lost results');
			const pair = results[0];
			if (!pair || typeof pair !== 'object' || Array.isArray(pair))
				throw new Error('Fixture lost pair');
			pair[field] = 'planted_wrong_value';
			expect(() => requireRulebookCompatibility(report, changed)).toThrow();
		}
	});
	it('rejects altered request counts, uncertainty views and result order', () => {
		expect(() => requireRulebookCompatibility(report, { ...report, requests: 2 })).toThrow();
		const changed = structuredClone(report);
		changed.report['uncertain'] = [];
		expect(() => requireRulebookCompatibility(report, changed)).toThrow();
		const before = structuredClone(report);
		before.report['results'] = [
			{ outcome: 'pass', step: 'a' },
			{ outcome: 'uncertain', step: 'b' },
		];
		const after = structuredClone(before);
		after.report['results'] = [
			{ outcome: 'uncertain', step: 'b' },
			{ outcome: 'pass', step: 'a' },
		];
		expect(() => requireRulebookCompatibility(before, after)).toThrow();
	});
	it('allows only the exact authored R44 reason delta, preserving incomplete identity', () => {
		const before = structuredClone(report);
		before.report['incomplete'] = [
			{ rule: 'rule', file: 'SKILL.md', reason: { kind: 'provider_failure' } },
		];
		const after = structuredClone(before);
		after.report['incomplete'] = [
			{
				rule: 'rule',
				file: 'SKILL.md',
				reason: { kind: 'provider_failure', failure_class: 'rate_limit', http_status: 429 },
			},
		];
		const allowed = { kind: 'provider_failure', failure_class: 'rate_limit', http_status: 429 };
		expect(requireRulebookCompatibility(before, after, allowed)).toHaveLength(1);
		expect(() => requireRulebookCompatibility(before, after)).toThrow();
		expect(() =>
			requireRulebookCompatibility(before, after, { kind: 'invalid_response' }),
		).toThrow();
		after.report['incomplete'] = [{ rule: 'different', file: 'SKILL.md', reason: allowed }];
		expect(() => requireRulebookCompatibility(before, after, allowed)).toThrow();
	});
});
