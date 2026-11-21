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

Use `conversation` with the original transcript path and its registered format.

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
