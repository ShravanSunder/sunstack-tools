/** Provider harness controls; synthetic receipts never establish ordinary-provider proof. */
import { describe, expect, it } from 'vitest';

import type {
	DecisionRunOutcome,
	LintStatus,
	SavedRequest,
	UsageSummary,
} from '../../../../packages/sdk/src/contracts/contracts.ts';
import {
	requireSdkProviderReport,
	sdkProviderConfiguration,
	sdkProviderLimits,
	sdkProviderWaitMilliseconds,
} from './sdk-provider.ts';

type CompletedProviderRequest = Extract<SavedRequest, { state: 'run' }> & {
	outcome: Extract<DecisionRunOutcome, { kind: 'completed' }>;
};
interface ProviderReportFixture {
	status: LintStatus;
	limits: typeof sdkProviderLimits & { retry_count: number };
	requests: CompletedProviderRequest[];
	usage: UsageSummary;
}

const providerReceiptFixture: ProviderReportFixture = {
	status: { kind: 'clean' },
	limits: { ...sdkProviderLimits, retry_count: 2 },
	requests: [
		{
			id: 0n,
			owner: { kind: 'filter', filter: 'purpose', folder: 'skills/portable' },
			requested_model: 'jev-latest',
			resolved_model: 'provider/model',
			state: 'run',
			request: {
				state: { body: 'Synthetic provider fixture.' },
				questions: {
					purpose: {
						type: 'choice',
						instructions: 'Classify reusable guidance.',
						criteria: { portable: 'Reusable.', local: 'Local.' },
					},
				},
			},
			outcome: {
				kind: 'completed',
				answers: {
					purpose: {
						type: 'choice',
						selected_label: 'portable',
						confidence: 1,
						probabilities: { portable: 1, local: 0 },
					},
				},
				receipt: {
					resolved_model: 'provider/model',
					usage: { input_tokens: 9007199254740993n, output_tokens: 5n },
				},
				attempts: [{ ordinal: 1, evidence: { kind: 'completed', http_status: 200 } }],
			},
		},
	],
	usage: {
		checked_files_acquired: 1n,
		model_requests: 1,
		input_tokens: 9007199254740993n,
		output_tokens: 5n,
		cost_usd: null,
		cost_reported_requests: 0,
		cost_missing_requests: 1,
	},
};
function providerReport(): ProviderReportFixture {
	return structuredClone(providerReceiptFixture);
}

describe('SDK ordinary-provider apparatus', () => {
	it('accepts actual top-level run/completed provider receipts with exact u64 usage and optional cost', () => {
		const report = providerReport();
		expect(() => requireSdkProviderReport(report, 0)).not.toThrow();
		const twoRequests = providerReport();
		const second = structuredClone(twoRequests.requests[0]);
		if (!second) throw new Error('Missing positive request fixture');
		second.id = 1n;
		twoRequests.requests.push(second);
		Object.assign(twoRequests.usage, {
			model_requests: 2,
			input_tokens: 18014398509481986n,
			output_tokens: 10n,
			cost_missing_requests: 2,
		});
		expect(() => requireSdkProviderReport(twoRequests, 0)).not.toThrow();
		for (const cost of [undefined, null, 0, 0.000001]) {
			const changed = structuredClone(report);
			const request = changed.requests[0];
			if (!request) throw new Error('Missing positive request fixture');
			if (cost === undefined) delete request.outcome.receipt.usage.cost_usd;
			else request.outcome.receipt.usage.cost_usd = cost;
			changed.usage.cost_usd = cost ?? null;
			changed.usage.cost_reported_requests = cost === undefined || cost === null ? 0 : 1;
			changed.usage.cost_missing_requests = 1 - changed.usage.cost_reported_requests;
			expect(() => requireSdkProviderReport(changed, 0)).not.toThrow();
		}
		const retried = providerReport();
		const request = retried.requests[0];
		if (!request) throw new Error('Missing positive request fixture');
		request.outcome.attempts = [
			{ ordinal: 1, evidence: { kind: 'remote_outcome_unknown', failure_class: 'timeout' } },
			{ ordinal: 2, evidence: { kind: 'completed', http_status: 200 } },
		];
		expect(() => requireSdkProviderReport(retried, 0)).not.toThrow();
	});
	it('retains exact bigint request IDs at zero, beyond 2^53 and u64 MAX', () => {
		for (const id of [0n, 1n, 9007199254740993n, 18446744073709551615n]) {
			const report = providerReport();
			const request = report.requests[0];
			if (!request) throw new Error('Missing positive request fixture');
			request.id = id;
			expect(() => requireSdkProviderReport(report, 0)).not.toThrow();
		}
	});
	it('refuses Number, negative, overflowing and duplicate provider request IDs', () => {
		const report = providerReport();
		const request = report.requests[0];
		if (!request) throw new Error('Missing positive request fixture');
		for (const id of [0, 1, 9007199254740992, -1n, 18446744073709551616n]) {
			expect(() =>
				requireSdkProviderReport({ ...report, requests: [{ ...request, id }] }, 0),
			).toThrow();
		}
		expect(() =>
			requireSdkProviderReport({ ...report, requests: [request, structuredClone(request)] }, 0),
		).toThrow('duplicated');
	});
	it('rejects prepared/reused/nested states, incomplete, controlled or malformed provider evidence', () => {
		const report = providerReport();
		const request = report.requests[0];
		expect(request).toBeDefined();
		for (const state of ['prepared', 'from_report', undefined])
			expect(() =>
				requireSdkProviderReport({ ...report, requests: [{ ...request, state }] }, 0),
			).toThrow();
		for (const wrong of [
			{ ...report, status: { kind: 'incomplete' } },
			{ ...report, requests: [] },
			{
				...report,
				requests: [{ ...request, outcome: { state: 'run', outcome: request?.outcome } }],
			},
			{
				...report,
				requests: [{ ...request, outcome: { ...request?.outcome, kind: 'cancelled' } }],
			},
			{ ...report, requests: [{ ...request, resolved_model: 'typesafe/controlled' }] },
			{ ...report, requests: [{ ...request, resolved_model: ' provider/model ' }] },
			{ ...report, requests: [{ ...request, outcome: { ...request?.outcome, receipt: {} } }] },
			{ ...report, requests: [{ ...request, outcome: { ...request?.outcome, attempts: [] } }] },
			{
				...report,
				requests: [
					{
						...request,
						outcome: {
							...request?.outcome,
							attempts: [{ ordinal: 2, evidence: { kind: 'completed', http_status: 200 } }],
						},
					},
				],
			},
			{
				...report,
				requests: [
					{
						...request,
						outcome: {
							...request?.outcome,
							attempts: [
								{
									ordinal: 1,
									evidence: { kind: 'remote_outcome_unknown', failure_class: 'timeout' },
								},
							],
						},
					},
				],
			},
			{ ...report, usage: { ...report.usage, input_tokens: 9007199254740992n } },
			{ ...report, requests: Array.from({ length: 3 }, () => request) },
		])
			expect(() => requireSdkProviderReport(wrong, 0)).toThrow();
		for (const input_tokens of [1, '9007199254740993', -1n, 18446744073709551616n]) {
			const wrong = {
				...request,
				outcome: {
					...request?.outcome,
					receipt: {
						...request?.outcome.receipt,
						usage: { input_tokens, output_tokens: 5n },
					},
				},
			};
			expect(() => requireSdkProviderReport({ ...report, requests: [wrong] }, 0)).toThrow();
		}
		expect(() => requireSdkProviderReport(report, 1)).toThrow('controlled backend');
	});
	it('bounds only the provider fixture using real public limits and no endpoint or public retry override', () => {
		const configuration = sdkProviderConfiguration('/synthetic');
		expect(configuration).toMatchObject({
			decision_model: { model: 'jev-latest' },
			limits: {
				max_files: 1,
				max_matches: 1,
				max_model_requests: 2,
				max_concurrent_requests: 1,
				attempt_timeout_seconds: 20,
				max_input_tokens_per_request: 4096,
				request_max_chars: 8000,
			},
		});
		expect(
			JSON.stringify(configuration, (_key, value: unknown) =>
				typeof value === 'bigint' ? String(value) : value,
			),
		).not.toContain('endpoint');
		expect(sdkProviderLimits).not.toHaveProperty('retry_count');
		expect(sdkProviderWaitMilliseconds).toBe(150000);
	});
});
