import type { RuleRunProfile } from '../../src/rules.ts';
import { document, evaluateRules, loadRuleSet, prepareRules } from '../../src/rules.ts';

declare const profile: RuleRunProfile;

const loaded = await loadRuleSet({
	path: 'set.json',
	inputContract: { changed: { kind: 'document' } },
	profile,
});
if (loaded.kind === 'ready') {
	const invocation = {
		inputs: { changed: document({ repository: 'app', path: 'sample.ts' }) },
		origin: { kind: 'direct' },
	} as const;
	await prepareRules(loaded.ruleSet, invocation);
	const result = await evaluateRules(loaded.ruleSet, invocation);
	if (result.kind === 'complete_report') {
		const requests: bigint = result.report.measurements.model_requests;
		void requests;
	}
}
