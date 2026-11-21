import { AdapterRegistry, conversation } from '../../src/rules.ts';
import type { RuleCallOptions, TranscriptAdapterMapping } from '../../src/rules.ts';

const mapped = {
	format: 'example_jsonl',
	discriminator: '/event',
	records: { answer: { kind: 'assistant_message', text: '/body' } },
	schema: { type: 'object', required: ['body'] },
} as const satisfies TranscriptAdapterMapping;
const registry = AdapterRegistry.builtins()
	.withAdapter(mapped)
	.withAdapter({
		format: 'second_jsonl',
		discriminator: '/kind',
		records: { done: { kind: 'tool_result', call_id: '/id', text: '/text', is_error: '/failed' } },
	});
const custom: 'example_jsonl' = registry.conversation({
	path: '/trace',
	format: mapped.format,
}).format;
const second: 'second_jsonl' = registry.conversation({
	path: '/trace',
	format: 'second_jsonl',
}).format;
const builtin: 'claude_code_jsonl' = registry.conversation({
	path: '/trace',
	format: 'claude_code_jsonl',
}).format;
const plain: 'codex_rollout_jsonl' = conversation({
	path: '/trace',
	format: 'codex_rollout_jsonl',
}).format;
const options: RuleCallOptions = { adapterRegistry: registry };
export { builtin, custom, options, plain, second };
