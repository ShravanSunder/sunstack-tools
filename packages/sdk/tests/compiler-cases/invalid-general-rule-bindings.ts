import type { BoundRuleInvocation } from '../../src/rules.ts';
import { AdapterRegistry, conversation, document } from '../../src/rules.ts';

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

conversation({
	path: 'session.jsonl',
	// expect TS2322
	format: 'codex_jsonl',
});

const mapping = {
	format: 'example_jsonl',
	discriminator: '/event',
	records: { answer: { kind: 'assistant_message', text: '/body' } },
} as const;
const builtins = AdapterRegistry.builtins();
const registered = builtins.withAdapter(mapping);

// expect TS2322
registered.conversation({ path: '/trace', format: 'missing_jsonl' });
// expect TS2322
builtins.conversation({ path: '/trace', format: 'example_jsonl' });
// expect TS2345
registered.withAdapter(mapping);
// expect TS2322
registered.withAdapter({ ...mapping, format: 'codex_rollout_jsonl' });
registered.withAdapter({
	format: 'invalid_jsonl',
	discriminator: '/event',
	// expect TS2322
	records: { broken: { kind: 'tool_result', text: '/body' } },
});

export function ambiguousRegistration(
	union: 'first_jsonl' | 'second_jsonl',
	wide: string,
	open: `example_${number}`,
): void {
	// expect TS2322
	registered.withAdapter({ ...mapping, format: union });
	// expect TS2322
	registered.withAdapter({ ...mapping, format: wide });
	// expect TS2322
	registered.withAdapter({ ...mapping, format: open });
}
