# Evaluate native rules over code or a conversation

Install `@sunstack/sdk` with its matching platform addon. The `@sunstack/sdk/rules` import works in Node 26 and in Bun with exact JSON support. Use the same rule files you pass to `sunstack exec`.

## Load once and evaluate when an event arrives

```ts
import { document, evaluateRules, loadRuleSet } from '@sunstack/sdk/rules';

import { profile } from './application-profile.ts';

const loaded = await loadRuleSet({
	path: 'general-document.ruleset.json',
	inputContract: { subject: { kind: 'document' } },
	profile,
});

export async function onChangedFile(path: string, signal: AbortSignal) {
	if (loaded.kind !== 'ready') return loaded;
	return evaluateRules(
		loaded.ruleSet,
		{
			inputs: { subject: document({ repository: 'example', path }) },
			origin: { kind: 'direct' },
		},
		{ signal },
	);
}
```

Use `conversation` with the original transcript path for built-in formats.

| Transcript          | `format`              |
| ------------------- | --------------------- |
| Codex rollout JSONL | `codex_rollout_jsonl` |
| Claude Code JSONL   | `claude_code_jsonl`   |

`builtinTranscriptFormats` exports the names from the same Rust list that registers the adapters.

## Runnable program

Copy the two `general-*.ruleset.json` files, their `general-*.rules.jsonl` files, `general-sample.ts`, and `general-codex-session.jsonl` from this examples folder into a fixture directory. Save the program below as `general-rules.mts` in your project. The `.mts` extension selects ESM under NodeNext, including in a default npm project.

`loadRuleSet` captures the rule set and its rules. The input companion names the bindings each run must supply. The profile makes file access, model choice, retries, and budgets explicit. The document rule uses a paragraph as its judgment target and attaches the exact TypeScript AST match as evidence. The conversation rule selects tool calls and their observed results from the Codex rollout.

```ts
import { resolve } from 'node:path';

import type { EvidencePlanInput, RuleRunProfile } from '@sunstack/sdk/rules';
import {
	conversation,
	document,
	evaluateRules,
	inspectEvidence,
	loadRuleSet,
	prepareRules,
	serialize,
} from '@sunstack/sdk/rules';

const [, , directory, sourceKind, action, transcriptPath] = process.argv;
if (directory === undefined || (sourceKind !== 'document' && sourceKind !== 'conversation')) {
	throw new Error(
		'Pass a fixture directory, document or conversation, and inspect, preview, or evaluate.',
	);
}
if (action !== 'inspect' && action !== 'preview' && action !== 'evaluate') {
	throw new Error('Choose inspect, preview, or evaluate.');
}
const root = resolve(directory);
const subject =
	sourceKind === 'document'
		? document({ repository: 'example', path: 'general-sample.ts' })
		: conversation({
				path:
					transcriptPath === undefined
						? resolve(root, 'general-codex-session.jsonl')
						: resolve(transcriptPath),
				format: 'codex_rollout_jsonl',
			});
const repositories: RuleRunProfile['repositories'] =
	sourceKind === 'document'
		? [
				{
					key: 'example',
					root,
					input_paths: [{ kind: 'exact', path: 'general-sample.ts' }],
					change_scope: { kind: 'full' },
				},
			]
		: [];
const limits: RuleRunProfile['limits'] = {
	max_files: 8n,
	max_file_bytes: 65_536n,
	max_total_bytes: 131_072n,
	max_matches: 20n,
	max_transcript_bytes: 65_536n,
	include_ignored: false,
};

if (action === 'inspect') {
	const selector: EvidencePlanInput['declarations'][number]['select'] =
		sourceKind === 'document'
			? { kind: 'ast_pattern', language: 'TypeScript', pattern: 'process.stdout.write($$$ARGS)' }
			: {
					kind: 'conversation',
					turns: { kind: 'all' },
					user_messages: 'excluded',
					assistant_messages: 'excluded',
					tool_use: { kind: 'included', tools: { kind: 'all' }, results: 'included' },
					hook_injected: 'excluded',
				};
	const outcome = await inspectEvidence({
		repositories,
		sources: { subject },
		payload_schemas: [],
		cursors: [],
		declarations: [{ id: 'selected', input: 'subject', requirement: 'required', select: selector }],
		native_queries: [],
		package_bindings: [],
		limits,
		discovery: { kind: 'none' },
	});
	if (outcome.kind === 'failed') throw new Error(outcome.failure.message);
	const saved = serialize({ kind: 'evidence_plan_outcome', value: outcome });
	if (saved.kind === 'rejected') throw new Error(saved.message);
	console.log(saved.json);
} else {
	const endpoint = process.env['SUNSTACK_JEV_ENDPOINT'];
	if (endpoint === undefined)
		throw new Error('Set SUNSTACK_JEV_ENDPOINT for preview or evaluation.');
	const profile: RuleRunProfile = {
		definition_root: root,
		repositories,
		discovery: { kind: 'none' },
		package_bindings: [],
		payload_schemas: [],
		cursors: [],
		execution: { speculation: 'off', max_concurrent_requests: 2n },
		limits,
		backend: { kind: 'jev_openrouter', endpoint, model: 'jev-latest' },
		attempts: {
			attempt_timeout_ms: 5_000n,
			retry_count: 0,
			initial_backoff_ms: 100n,
			max_backoff_ms: 1_000n,
			max_response_bytes: 65_536n,
		},
		request_budget: {
			max_input_tokens: 100_000n,
			max_request_bytes: 100_000n,
			max_questions: 20n,
			max_requests: 4n,
			safety_margin_tokens: 32n,
			accounting_policy: 'allow_estimated',
		},
	};
	const loaded = await loadRuleSet({
		path: `general-${sourceKind}.ruleset.json`,
		inputContract: { subject: { kind: subject.kind } },
		profile,
	});
	if (loaded.kind !== 'ready') throw new Error(`Rule admission ended with ${loaded.kind}.`);
	const invocation = { inputs: { subject }, origin: { kind: 'direct' } } as const;
	if (action === 'preview') {
		const preview = await prepareRules(loaded.ruleSet, invocation);
		if (preview.kind !== 'ready_single') throw new Error(`Preview ended with ${preview.kind}.`);
		console.log(preview.request.body);
	} else {
		const outcome = await evaluateRules(loaded.ruleSet, invocation);
		if (outcome.kind === 'failed') throw new Error(outcome.failure.message);
		const saved = serialize({ kind: 'rule_invocation_outcome', value: outcome });
		if (saved.kind === 'rejected') throw new Error(saved.message);
		console.log(saved.json);
	}
}
```

Save the companion reader below as `read-codex-calls.mts`. With TypeScript and Node types installed, check both programs with this command.

```sh
npx tsc --ignoreConfig --noEmit --strict --target es2024 --module nodenext --types node general-rules.mts read-codex-calls.mts
```

Run `node general-rules.mts ./fixtures document inspect` or `bun general-rules.mts ./fixtures conversation inspect` with no backend settings. For `preview`, set `SUNSTACK_JEV_ENDPOINT` to the target you want to inspect. Preview needs neither a live server nor a credential. Use `evaluate` with `OPENROUTER_API_KEY` when you want to send the request. A local responder can use a synthetic credential.

For evaluation, read the outcome tag first. A `complete_report` has completed evaluation and can contain a favorable or unfavorable judgment. An `incomplete_report`, `cancelled`, or `invalid` has its own reason or coverage. Inspect `report.occurrences` to see which targets were judged. Keep the serialized SDK document and pass its original text to `deserialize` to restore exact integer fields as `bigint`.

## Read the selected Codex calls

The fixture selects two calls. In a real rollout the same selection can return many calls, an empty list, or partial evidence. Read every selected record. Use the capture coverage and result tags to see what the transcript supplied. A call with `result_missing` is still a call. Keep the original `tool_name`, including its namespace, and `call_id`.

```sh
node general-rules.mts ./fixtures conversation inspect > conversation.sdk.json
node read-codex-calls.mts conversation.sdk.json
```

Append the path to your original Codex rollout after `inspect` to use it in place of the fixture.

```ts
import { readFileSync } from 'node:fs';

import { deserialize } from '@sunstack/sdk/rules';

const [, , reportPath] = process.argv;
if (reportPath === undefined) throw new Error('Pass the saved evidence document.');
const restored = await deserialize(readFileSync(reportPath, 'utf8'));
if (restored.kind !== 'decoded' || restored.value.kind !== 'evidence_plan_outcome') {
	throw new Error('Expected an SDK evidence document from this revision.');
}

const outcome = restored.value.value;
const selections =
	outcome.kind === 'prepared' || outcome.kind === 'incomplete'
		? outcome.selections
		: outcome.kind === 'cancelled' && outcome.progress.kind === 'plan_admitted'
			? outcome.progress.partial.selections
			: [];

const calls = selections.flatMap(({ readiness }) => {
	if (readiness.kind !== 'ready' || readiness.content.kind !== 'conversation_records') return [];
	return readiness.content.records.flatMap((record) => {
		if (record.body.kind !== 'tool_use') return [];
		const { call_id, tool_name, outcome, result } = record.body;
		return [
			{
				call_id,
				tool_name,
				provenance: record.provenance,
				outcome,
				result: result.kind,
			},
		];
	});
});

const coverage = selections.map(({ id, readiness }) => {
	if (readiness.kind === 'refused') return { id, kind: readiness.kind };
	const capture =
		readiness.coverage.kind === 'single_source' ? readiness.coverage.capture : undefined;
	return {
		id,
		kind: readiness.kind,
		tail: capture?.tail.kind,
		gaps: capture?.gaps.map((gap) => gap.kind),
		omissions: readiness.omissions.map((omission) => omission.reason),
	};
});

console.log(JSON.stringify({ kind: outcome.kind, calls, coverage }));
```

## Register another JSONL transcript

Use `AdapterRegistry.builtins().withAdapter(mapping)` to create a snapshot with an additional format. Define the discriminator and each record field with JSON Pointers. Bind the original file with `adapters.conversation({ path, format })` and pass `{ adapterRegistry: adapters }` to `inspectEvidence`, `prepareRules`, or `evaluateRules`. The adapter accepts string message text and call IDs, preserves tool inputs as JSON values, and interprets a supplied `is_error` field as a boolean. An omitted result status remains unknown.

Each registration takes one concrete format-name literal. When a name comes from a union, narrow it to one branch before calling `withAdapter`.

You can supply an offline Draft 2020-12 JSON Schema as the optional `schema` property, including a schema emitted by Zod's JSON Schema conversion. The schema validates selected rows, and the record pointers extract their fields. Inspect the returned coverage for unsupported or malformed rows, orphan results, and uncertainty about user-message injection. A changed adapter mapping or schema requires a fresh cursor.

When a native message ID and a generated offset or tool record ID collide, the first record keeps its identity. The later row appears as a malformed-record gap at its original position.

The program below binds its synthetic JSONL and a Codex rollout in the same request. Save it as `transcript-adapter.mts` and copy the fixtures `transcript-adapter.jsonl`, `transcript-adapter.ruleset.json`, `transcript-adapter.rules.jsonl`, and `general-codex-session.jsonl` into your consumer project. Run these commands from that directory.

```sh
npx tsc --ignoreConfig --noEmit --strict --module NodeNext --target ES2024 --types node transcript-adapter.mts
node transcript-adapter.mts . inspect
SUNSTACK_JEV_ENDPOINT=http://127.0.0.1:4199/v1/systemone node transcript-adapter.mts . preview
```

Preview prepares one request locally and prints its body without contacting the endpoint. Use `evaluate` with an endpoint and credentials already configured when you want to submit the request.

```ts
import { resolve } from 'node:path';

import type { EvidencePlanInput, RuleRunProfile } from '@sunstack/sdk/rules';
import {
	AdapterRegistry,
	conversation,
	deserialize,
	evaluateRules,
	inspectEvidence,
	loadRuleSet,
	prepareRules,
	serialize,
} from '@sunstack/sdk/rules';

const [, , fixtureDirectory, action, transcriptPath] = process.argv;
if (
	fixtureDirectory === undefined ||
	(action !== 'inspect' && action !== 'preview' && action !== 'evaluate')
) {
	throw new Error('Pass a fixture directory and inspect, preview, or evaluate.');
}
const root = resolve(fixtureDirectory);
const adapters = AdapterRegistry.builtins().withAdapter({
	format: 'example_jsonl',
	discriminator: '/event',
	records: {
		operator: { kind: 'user_message', message_id: '/message_id', text: '/body/text' },
		assistant: { kind: 'assistant_message', message_id: '/message_id', text: '/body/text' },
		tool_started: { kind: 'tool_call', call_id: '/id', tool_name: '/name', input: '/arguments' },
		tool_finished: { kind: 'tool_result', call_id: '/id', text: '/output', is_error: '/failed' },
	},
});
const inputs = {
	custom: adapters.conversation({
		path: resolve(transcriptPath ?? resolve(root, 'transcript-adapter.jsonl')),
		format: 'example_jsonl',
	}),
	codex: conversation({
		path: resolve(root, 'general-codex-session.jsonl'),
		format: 'codex_rollout_jsonl',
	}),
};
const options = { adapterRegistry: adapters };
const limits: RuleRunProfile['limits'] = {
	max_files: 8n,
	max_file_bytes: 65_536n,
	max_total_bytes: 131_072n,
	max_matches: 20n,
	max_transcript_bytes: 65_536n,
	include_ignored: false,
};

if (action === 'inspect') {
	const select: EvidencePlanInput['declarations'][number]['select'] = {
		kind: 'conversation',
		turns: { kind: 'all' },
		user_messages: 'included',
		assistant_messages: 'all',
		hook_injected: 'excluded',
		tool_use: { kind: 'included', tools: { kind: 'all' }, results: 'included' },
	};
	const outcome = await inspectEvidence(
		{
			repositories: [],
			payload_schemas: [],
			cursors: [],
			discovery: { kind: 'none' },
			package_bindings: [],
			native_queries: [],
			limits,
			sources: inputs,
			declarations: [
				{ id: 'custom', input: 'custom', requirement: 'required', select },
				{ id: 'codex', input: 'codex', requirement: 'required', select },
			],
		},
		options,
	);
	if (outcome.kind === 'failed') throw new Error(outcome.failure.message);
	const document = serialize({ kind: 'evidence_plan_outcome', value: outcome });
	if (document.kind !== 'serialized') throw new Error(document.message);
	const restored = await deserialize(document.json);
	if (restored.kind !== 'decoded') throw new Error(`Could not restore evidence: ${restored.kind}`);
	console.log(document.json);
} else {
	const endpoint = process.env['SUNSTACK_JEV_ENDPOINT'];
	if (endpoint === undefined)
		throw new Error('Set SUNSTACK_JEV_ENDPOINT for preview or evaluation.');
	const loaded = await loadRuleSet({
		path: 'transcript-adapter.ruleset.json',
		inputContract: { custom: { kind: 'conversation' }, codex: { kind: 'conversation' } },
		profile: {
			definition_root: root,
			repositories: [],
			discovery: { kind: 'none' },
			package_bindings: [],
			payload_schemas: [],
			cursors: [],
			limits,
			execution: { speculation: 'off', max_concurrent_requests: 1n },
			backend: { kind: 'jev_openrouter', endpoint, model: 'jev-latest' },
			attempts: {
				attempt_timeout_ms: 5_000n,
				retry_count: 0,
				initial_backoff_ms: 100n,
				max_backoff_ms: 1_000n,
				max_response_bytes: 65_536n,
			},
			request_budget: {
				max_input_tokens: 100_000n,
				max_request_bytes: 100_000n,
				max_questions: 20n,
				max_requests: 20n,
				safety_margin_tokens: 32n,
				accounting_policy: 'allow_estimated',
			},
		},
	});
	if (loaded.kind !== 'ready') throw new Error(`Rule set admission ended with ${loaded.kind}.`);
	const invocation = { inputs, origin: { kind: 'direct' } } as const;
	if (action === 'preview') {
		const outcome = await prepareRules(loaded.ruleSet, invocation, options);
		if (outcome.kind !== 'ready_single')
			throw new Error(`Expected one request; got ${outcome.kind}.`);
		console.log(outcome.request.body);
	} else {
		const outcome = await evaluateRules(loaded.ruleSet, invocation, options);
		if (outcome.kind === 'failed') throw new Error(outcome.failure.message);
		const document = serialize({ kind: 'rule_invocation_outcome', value: outcome });
		if (document.kind !== 'serialized') throw new Error(document.message);
		console.log(document.json);
	}
}
```
