import type { BoundRuleInvocation } from '../../src/rules.ts';
import { conversation, document } from '../../src/rules.ts';

type SourceContract = { readonly changed: { readonly kind: 'document' } };
export const unknownName: BoundRuleInvocation<SourceContract> = {
	inputs: {
		// expect TS2353
		edited: document({ repository: 'app', path: 'sample.ts' }),
	},
	origin: { kind: 'direct' },
};
export const wrongSource: BoundRuleInvocation<SourceContract> = {
	inputs: {
		// expect TS2739
		changed: conversation({ path: 'session.jsonl', format: 'codex_rollout_jsonl' }),
	},
	origin: { kind: 'direct' },
};
export const missingBinding: BoundRuleInvocation<SourceContract> = {
	// expect TS2741
	inputs: {},
	origin: { kind: 'direct' },
};
