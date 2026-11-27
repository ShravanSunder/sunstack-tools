import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

import type { EvidencePlanInput, RuleInvocationInput } from '../src/rules.ts';
import {
	AdapterRegistry,
	conversation,
	deserialize,
	evaluateRules,
	inspectEvidence,
	prepareRules,
	serialize,
} from '../src/rules.ts';
import { answerEveryRequest, startControlledBackend } from './support/controlled-backend.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

const adapterDefinition = {
	format: 'example_jsonl',
	discriminator: '/event',
	records: {
		operator: { kind: 'user_message', message_id: '/message_id', text: '/body/text' },
		assistant: { kind: 'assistant_message', message_id: '/message_id', text: '/body/text' },
		tool_started: { kind: 'tool_call', call_id: '/id', tool_name: '/name', input: '/arguments' },
		tool_finished: { kind: 'tool_result', call_id: '/id', text: '/output', is_error: '/failed' },
	},
} as const;

const limits = {
	max_files: 8n,
	max_file_bytes: 65_536n,
	max_total_bytes: 131_072n,
	max_matches: 20n,
	max_transcript_bytes: 65_536n,
	include_ignored: false,
} as const;
const select = {
	kind: 'conversation',
	turns: { kind: 'all' },
	user_messages: 'included',
	assistant_messages: 'all',
	hook_injected: 'excluded',
	tool_use: { kind: 'included', tools: { kind: 'all' }, results: 'included' },
} as const;

function packagedFixture(name: string): string {
	return readFileSync(new URL(`../examples/${name}`, import.meta.url), 'utf8');
}

void test(
	'registered snapshot survives caller edits and powers evidence, preview, evaluation and codec',
	{ timeout: 30_000 },
	async () => {
		const root = ownedDirectory('registered-transcript');
		const fixture = packagedFixture('transcript-adapter.jsonl');
		const allowedInteger = 9_007_199_254_740_993n;
		const allowedRow = `{"event":"assistant","message_id":"schema-accepted","body":{"text":"schema accepted"},"fixtureAllowed":${allowedInteger}}\n`;
		const refusedRow = `{"event":"assistant","body":{"text":"schema rejected"},"fixtureAllowed":${allowedInteger - 1n}}\n`;
		const trace = `${fixture}${allowedRow}${refusedRow}`;
		const backend = await startControlledBackend();
		const responder = answerEveryRequest(backend, 1);
		try {
			process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';
			delete process.env['OPENROUTER_API_KEY'];
			writeFiles(root.path, {
				'trace.jsonl': trace,
				'codex.jsonl': packagedFixture('general-codex-session.jsonl'),
				'transcript-adapter.ruleset.json': packagedFixture('transcript-adapter.ruleset.json'),
				'transcript-adapter.rules.jsonl': packagedFixture('transcript-adapter.rules.jsonl'),
			});
			const subschema = Object.defineProperty({ const: allowedInteger }, '~standard', {
				value: { validate: () => false },
			});
			const editable = {
				...adapterDefinition,
				schema: {
					type: 'object',
					properties: { fixtureAllowed: { anyOf: [subschema, subschema] } },
				},
				records: {
					...adapterDefinition.records,
					tool_finished: { ...adapterDefinition.records.tool_finished, text: '/output' },
				},
			};
			const builtins = AdapterRegistry.builtins();
			let schemaReads = 0;
			const accessorMapping = Object.defineProperty({ ...adapterDefinition }, 'schema', {
				get() {
					schemaReads += 1;
					return { type: 'object' };
				},
			});
			assert.throws(() => builtins.withAdapter(accessorMapping), /schema must be a data property/u);
			assert.equal(schemaReads, 0);
			const registry = builtins.withAdapter(editable);
			editable.records.tool_finished.text = '/wrong';
			const register = registry.withAdapter.bind(registry);
			const enumerableMetadata = Object.defineProperty({ type: 'object' }, '~standard', {
				enumerable: true,
				value: { validate: () => false },
			});
			assert.throws(
				() => builtins.withAdapter({ ...adapterDefinition, schema: enumerableMetadata }),
				/function, not data/u,
			);
			assert.throws(
				() => Reflect.apply(register, registry, [adapterDefinition]),
				/already registered/u,
			);
			assert.throws(
				() =>
					Reflect.apply(register, registry, [
						{ ...adapterDefinition, format: 'codex_rollout_jsonl' },
					]),
				/already registered/u,
			);
			const inputs = {
				custom: registry.conversation({
					path: join(root.path, 'trace.jsonl'),
					format: 'example_jsonl',
				}),
				codex: conversation({
					path: join(root.path, 'codex.jsonl'),
					format: 'codex_rollout_jsonl',
				}),
			};
			const plan: EvidencePlanInput = {
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
			};
			const noRegistration = await inspectEvidence(plan, { adapterRegistry: builtins });
			assert.equal(noRegistration.kind, 'refused');
			assert.equal(noRegistration.diagnostics.items[0]?.code, 'unknown_transcript_format');
			const caughtByNativeAdmission = AdapterRegistry.builtins().withAdapter({
				...adapterDefinition,
				discriminator: '/invalid~q',
			});
			const absentFile = { ...inputs.custom, transcript_path: join(root.path, 'missing.jsonl') };
			const refused = await inspectEvidence(
				{ ...plan, sources: { ...inputs, custom: absentFile } },
				{
					adapterRegistry: caughtByNativeAdmission,
				},
			);
			assert.equal(refused.kind, 'refused');
			assert.equal(refused.diagnostics.items[0]?.code, 'invalid_pointer');

			const options = { adapterRegistry: registry };
			const evidence = await inspectEvidence(plan, options);
			assert.equal(evidence.kind, 'prepared');
			const custom = evidence.selections.find(({ id }) => id === 'custom')?.readiness;
			assert.equal(custom?.kind, 'ready');
			assert.equal(custom.content.kind, 'conversation_records');
			const [user, call, assistant, pending, validated] = custom.content.records;
			assert.equal(user?.record_id, 'example-user');
			assert.equal(user.injection, 'unknown');
			assert.equal(assistant?.record_id, 'example-assistant');
			assert.equal(call?.body.kind, 'tool_use');
			assert.equal(
				call.position,
				BigInt(Buffer.byteLength(trace.split('\n').slice(0, 2).join('\n')) + 1),
			);
			assert.equal(call.body.call_id, 'example-call');
			assert.deepEqual(call.body.input, { path: 'sample.txt', sequence: 9007199254740993n });
			assert.deepEqual(call.body.result, {
				kind: 'full_observed',
				text: 'EXAMPLE_SELECTED_RESULT',
			});
			assert.equal(pending?.body.kind, 'tool_use');
			assert.equal(pending.body.outcome, 'result_missing');
			assert.equal(validated?.record_id, 'schema-accepted');
			assert.equal(custom.content.records.length, 5);
			assert.equal(custom.coverage.kind, 'single_source');
			assert.ok(
				custom.coverage.capture.gaps.some(
					(gap) =>
						gap.kind === 'malformed_record' &&
						gap.position === BigInt(Buffer.byteLength(fixture + allowedRow)),
				),
			);
			const saved = serialize({ kind: 'evidence_plan_outcome', value: evidence });
			assert.equal(saved.kind, 'serialized');
			assert.deepEqual(await deserialize(saved.json), {
				kind: 'decoded',
				value: { kind: 'evidence_plan_outcome', value: evidence },
			});

			const invocation: RuleInvocationInput = {
				definition_root: root.path,
				definitions: {
					kind: 'rule_sets',
					rule_sets: [
						{ path: 'transcript-adapter.ruleset.json', binding_groups: [{ bindings: inputs }] },
					],
				},
				repositories: [],
				payload_schemas: [],
				cursors: [],
				origin: { kind: 'direct' },
				limits,
				execution: { speculation: 'off', max_concurrent_requests: 1n },
				backend: { kind: 'jev_openrouter', endpoint: backend.endpoint, model: 'jev-latest' },
				attempts: {
					attempt_timeout_ms: 4_000n,
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
			};
			const preview = await prepareRules(invocation, options);
			assert.equal(preview.kind, 'ready_single');
			assert.equal(backend.received.length, 0);
			process.env['OPENROUTER_API_KEY'] = 'synthetic-transcript-adapters-test';
			const evaluated = await evaluateRules(invocation, options);
			assert.equal(evaluated.kind, 'complete_report');
			assert.equal(evaluated.report.measurements.model_requests, 1n);
			assert.equal(backend.received.length, 1);
			const body = backend.received[0]?.rawBody.toString('utf8');
			assert.equal(body, preview.request.body);
			assert.match(body, /EXAMPLE_SELECTED_RESULT/u);
			assert.match(body, /example-pending/u);
			assert.match(body, /list_sources/u);
		} finally {
			delete process.env['OPENROUTER_API_KEY'];
			await responder.stop();
			await backend.close();
			root.remove();
		}
	},
);

void test('nested schema metadata does not hide accessors, executable data or cycles', () => {
	const builtins = AdapterRegistry.builtins();
	let reads = 0;
	const accessor = Object.defineProperty({}, 'const', {
		get() {
			reads += 1;
			return 'never read';
		},
	});
	const executable = Object.defineProperty({}, '~standard', {
		enumerable: true,
		value: { validate: () => false },
	});
	const cyclic = { allOf: [] };
	Object.defineProperty(cyclic, 'allOf', { value: [cyclic] });
	assert.throws(
		() => builtins.withAdapter({ ...adapterDefinition, schema: { anyOf: [accessor] } }),
		/schema must contain only data properties/u,
	);
	assert.equal(reads, 0);
	assert.throws(
		() => builtins.withAdapter({ ...adapterDefinition, schema: { anyOf: [executable] } }),
		/function, not data/u,
	);
	assert.throws(
		() => builtins.withAdapter({ ...adapterDefinition, schema: cyclic }),
		/is a cycle/u,
	);
});
