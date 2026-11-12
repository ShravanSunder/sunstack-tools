/** Ordinary-provider fixture bounds and real saved-request receipt assertions. */
import { z } from 'zod';

import { sdkInlineConfiguration } from './sdk-fixtures.ts';
import { requireSdk } from './sdk-target.ts';

export const sdkProviderWaitMilliseconds = 150_000;
export const sdkProviderLimits = Object.freeze({
	max_files: 1,
	max_file_bytes: 4096n,
	max_total_bytes: 4096n,
	max_matches: 1,
	max_model_requests: 2,
	max_concurrent_requests: 1,
	attempt_timeout_seconds: 20,
	max_input_tokens_per_request: 4096,
	request_max_chars: 8000,
	evidence_max_chars: 2000,
});

export function sdkProviderConfiguration(root: string): object {
	return {
		...sdkInlineConfiguration(root, 'unused'),
		decision_model: { model: 'jev-latest' },
		limits: sdkProviderLimits,
	};
}

const maximumU64 = 18446744073709551615n;
const u64 = z.bigint().min(0n).max(maximumU64);
const model = z
	.string()
	.min(1)
	.refine((value) => value.trim().length > 0 && !value.toLowerCase().includes('controlled'));
const cost = z.number().finite().nonnegative();
const failure = z.enum([
	'provider_rejected_request',
	'authentication',
	'authorization',
	'insufficient_credits',
	'provider_resource_unavailable',
	'request_too_large',
	'rate_limit',
	'provider_unavailable',
	'timeout',
	'connection',
	'http_error',
	'unexpected_provider_error',
	'invalid_response',
	'caller_aborted',
]);
const attemptEvidence = z.discriminatedUnion('kind', [
	z.strictObject({ kind: z.literal('completed'), http_status: z.number().int().min(200).max(299) }),
	z.strictObject({ kind: z.literal('not_dispatched'), failure_class: failure }),
	z.strictObject({
		kind: z.literal('remote_outcome_unknown'),
		failure_class: failure,
		http_status: z.number().int().min(0).max(65535).nullish(),
	}),
]);
const providerRequest = z.object({
	id: z.number().int().nonnegative(),
	state: z.literal('run'),
	resolved_model: model,
	request: z.object({ questions: z.record(z.string(), z.unknown()) }),
	outcome: z.strictObject({
		kind: z.literal('completed'),
		answers: z.record(z.string(), z.unknown()),
		receipt: z.strictObject({
			resolved_model: model,
			usage: z.strictObject({ input_tokens: u64, output_tokens: u64, cost_usd: cost.nullish() }),
		}),
		attempts: z
			.array(z.strictObject({ ordinal: z.number().int().positive(), evidence: attemptEvidence }))
			.min(1)
			.max(3),
	}),
});

export function requireSdkProviderReport(value: unknown, controlledCaptureCount: number): void {
	requireSdk(controlledCaptureCount === 0, 'Hosted journey reached controlled backend');
	const report = z
		.object({
			status: z.object({ kind: z.enum(['clean', 'nothing_selected', 'findings', 'uncertain']) }),
			requests: z.array(providerRequest).min(1).max(2),
			limits: z.object({
				retry_count: z.literal(2),
				max_model_requests: z.literal(2),
				max_concurrent_requests: z.literal(1),
				attempt_timeout_seconds: z.literal(20),
			}),
			usage: z.object({
				model_requests: z.number().int(),
				input_tokens: u64,
				output_tokens: u64,
				cost_usd: cost.nullable(),
				cost_reported_requests: z.number().int(),
				cost_missing_requests: z.number().int(),
			}),
		})
		.parse(value);
	requireSdk(
		new Set(report.requests.map((request) => request.id)).size === report.requests.length,
		'Provider request identities are duplicated',
	);
	let inputTokens = 0n;
	let outputTokens = 0n;
	let totalCost: number | null = null;
	let costReports = 0;
	for (const request of report.requests) {
		const { receipt, attempts, answers } = request.outcome;
		requireSdk(
			request.resolved_model === receipt.resolved_model,
			'Provider resolved model differs',
		);
		const questions = Object.keys(request.request.questions).toSorted();
		requireSdk(
			questions.length === 1 &&
				questions[0] === Object.keys(answers).toSorted()[0] &&
				Object.keys(answers).length === 1,
			'Provider request is missing its actual answer',
		);
		for (const [index, attempt] of attempts.entries()) {
			requireSdk(attempt.ordinal === index + 1, 'Provider attempt ordinal differs');
			requireSdk(
				(attempt.evidence.kind === 'completed') === (index === attempts.length - 1),
				'Provider attempts lack one final completed attempt',
			);
		}
		inputTokens =
			inputTokens + receipt.usage.input_tokens > maximumU64
				? maximumU64
				: inputTokens + receipt.usage.input_tokens;
		outputTokens =
			outputTokens + receipt.usage.output_tokens > maximumU64
				? maximumU64
				: outputTokens + receipt.usage.output_tokens;
		if (receipt.usage.cost_usd !== undefined && receipt.usage.cost_usd !== null) {
			totalCost = (totalCost ?? 0) + receipt.usage.cost_usd;
			costReports += 1;
		}
	}
	requireSdk(
		report.usage.model_requests === report.requests.length &&
			report.usage.input_tokens === inputTokens &&
			report.usage.output_tokens === outputTokens &&
			report.usage.cost_usd === totalCost &&
			report.usage.cost_reported_requests === costReports &&
			report.usage.cost_missing_requests === report.requests.length - costReports,
		'Provider usage summary differs from exact request receipts',
	);
}
