/** Complete encoded payload multiplicity plus the exercised filter-before-rule dependency. */
import { z } from 'zod';

import { requireSdk } from './sdk-target.ts';

function payloadCounts(payloads: readonly string[]): Map<string, number> {
	const counts = new Map<string, number>();
	for (const payload of payloads) counts.set(payload, (counts.get(payload) ?? 0) + 1);
	return counts;
}
function requireFixtureDependencies(payloads: readonly string[]): void {
	let filterCount = 0;
	let ruleCount = 0;
	let ruleStarted = false;
	for (const raw of payloads) {
		const decoded: unknown = JSON.parse(raw);
		const payload = z.object({ questions: z.record(z.string(), z.unknown()) }).parse(decoded);
		const questions = Object.keys(payload.questions);
		requireSdk(
			questions.length === 1,
			'Fixture owner sequence must remain a single question per request',
		);
		if (questions[0] === 'purpose') {
			requireSdk(!ruleStarted, 'Filter request arrived after dependent rule request');
			filterCount += 1;
		} else {
			requireSdk(questions[0] === 'actor', 'Unknown fixture request owner');
			ruleStarted = true;
			ruleCount += 1;
		}
	}
	requireSdk(filterCount > 0 && ruleCount > 0, 'Fixture filter/rule request path was not reached');
}
export function requireSdkRequestParity(
	sdkPayloads: readonly string[],
	cliPayloads: readonly string[],
): void {
	const sdk = payloadCounts(sdkPayloads);
	const cli = payloadCounts(cliPayloads);
	requireSdk(
		sdk.size === cli.size && sdkPayloads.length === cliPayloads.length,
		'CLI/SDK request payload multiplicity differs',
	);
	for (const [payload, count] of sdk)
		requireSdk(cli.get(payload) === count, 'CLI/SDK complete encoded request payload differs');
	requireFixtureDependencies(sdkPayloads);
	requireFixtureDependencies(cliPayloads);
}
