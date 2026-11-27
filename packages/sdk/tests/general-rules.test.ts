import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

import type { InputBinding, RuleInvocationInput } from '../src/rules.ts';
import {
	deserialize,
	evaluateRules,
	inspectEvidence,
	loadRuleSet,
	prepareRules,
	serialize,
} from '../src/rules.ts';
import { encodeForNative } from '../src/wire-encoding.ts';
import { answerEveryRequest, startControlledBackend } from './support/controlled-backend.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

const fixtures = {
	document: {
		source: 'sample.ts',
		content: 'console.log("DOCUMENT_SENTINEL");\n',
		sentinel: 'DOCUMENT_SENTINEL',
		selector: { kind: 'ast_pattern', language: 'TypeScript', pattern: 'console.log($$$ARGS)' },
	},
	conversation: {
		source: 'session.jsonl',
		content:
			'{"type":"user","uuid":"u1","message":{"role":"user","content":"CONVERSATION_SENTINEL"}}\n' +
			'{"type":"assistant","uuid":"a1","message":{"role":"assistant","content":[{"type":"text","text":"Reply"}]}}\n',
		sentinel: 'CONVERSATION_SENTINEL',
		selector: {
			kind: 'conversation',
			turns: { kind: 'all' },
			user_messages: 'included',
			assistant_messages: 'excluded',
			tool_use: { kind: 'excluded' },
			hook_injected: 'excluded',
		},
	},
} as const;

function record(value: unknown): Readonly<Record<string, unknown>> {
	assert.ok(typeof value === 'object' && value !== null && !Array.isArray(value));
	return Object.fromEntries(Object.entries(value));
}

for (const sourceKind of ['document', 'conversation'] as const) {
	void test(
		`general SDK evaluates a ${sourceKind} through the native addon`,
		{ timeout: 30000 },
		async (context) => {
			const fixture = fixtures[sourceKind];
			const directory = ownedDirectory('general-rules');
			const backend = await startControlledBackend();
			const matchingCli = process.env['SUNSTACK_SDK_MATCHING_CLI'];
			const responder = answerEveryRequest(backend, matchingCli === undefined ? 1 : 2);
			try {
				writeFiles(directory.path, {
					[fixture.source]: fixture.content,
					'set.json': JSON.stringify({
						schema_version: 1,
						name: 'fixture',
						description: 'General rules SDK fixture',
						inputs: { subject: { kind: sourceKind } },
						applicability: [],
						common_ruleset_instructions: null,
						rule_paths: ['rules.jsonl'],
					}),
					'rules.jsonl': JSON.stringify({
						rule_id: 'selected',
						evidence:
							sourceKind === 'document'
								? [
										{
											id: 'target',
											input: 'subject',
											select: { kind: 'markdown_units', mode: 'unit', units: ['paragraph'] },
										},
										{ id: 'ast_matches', input: 'subject', select: fixture.selector },
									]
								: [{ id: 'target', input: 'subject', select: fixture.selector }],
						target: 'target',
						applicability: [],
						questions: {},
						verdict: {
							kind: 'noul',
							question: { type: 'noul', instructions: 'Check selected input' },
							threshold: 0.8,
						},
					}),
				});
				const subject: InputBinding =
					sourceKind === 'document'
						? { kind: 'document', repository: 'fixture', path: fixture.source }
						: {
								kind: 'conversation',
								transcript_path: join(directory.path, fixture.source),
								format: 'claude_code_jsonl',
							};
				const input: RuleInvocationInput = {
					definition_root: directory.path,
					definitions: {
						kind: 'rule_sets',
						rule_sets: [{ path: 'set.json', binding_groups: [{ bindings: { subject } }] }],
					},
					repositories:
						sourceKind === 'document'
							? [
									{
										key: 'fixture',
										root: directory.path,
										input_paths: [{ kind: 'exact', path: fixture.source }],
										change_scope: { kind: 'full' },
									},
								]
							: [],
					payload_schemas: [],
					cursors: [],
					origin: { kind: 'direct' },
					execution: { speculation: 'off', max_concurrent_requests: 2n },
					limits: {
						max_files: 8n,
						max_file_bytes: 65536n,
						max_total_bytes: 131072n,
						max_matches: 20n,
						max_transcript_bytes: 65536n,
						include_ignored: false,
					},
					backend: { kind: 'jev_openrouter', endpoint: backend.endpoint, model: 'jev-latest' },
					attempts: {
						attempt_timeout_ms: 4000n,
						retry_count: 0,
						initial_backoff_ms: 100n,
						max_backoff_ms: 1000n,
						max_response_bytes: 65536n,
					},
					request_budget: {
						max_input_tokens: 100000n,
						max_request_bytes: 100000n,
						max_questions: 20n,
						max_requests: 20n,
						safety_margin_tokens: 32n,
						accounting_policy: 'allow_estimated',
					},
				};

				delete process.env['OPENROUTER_API_KEY'];
				const inspected = await inspectEvidence({
					repositories: input.repositories,
					sources: { subject },
					payload_schemas: input.payload_schemas,
					cursors: input.cursors,
					declarations: [
						{ id: 'selected', input: 'subject', requirement: 'required', select: fixture.selector },
					],
					native_queries: [],
					package_bindings: [],
					limits: input.limits,
					discovery: { kind: 'none' },
				});
				assert.equal(inspected.kind, 'prepared');
				const evidence = inspected.selections[0]?.readiness;
				assert.equal(evidence?.kind, 'ready');
				assert.equal(
					evidence.content.kind,
					sourceKind === 'document' ? 'matches' : 'conversation_records',
				);
				const originalPreview = await prepareRules(input);
				assert.equal(originalPreview.kind, 'ready_single');

				const profile = {
					definition_root: input.definition_root,
					repositories: input.repositories,
					discovery: { kind: 'none' },
					package_bindings: [],
					payload_schemas: input.payload_schemas,
					cursors: input.cursors,
					execution: input.execution,
					limits: input.limits,
					backend: input.backend,
					attempts: input.attempts,
					request_budget: input.request_budget,
				} as const;
				if (sourceKind === 'document') {
					await checkOtherPreviewStates(input, directory.path, subject);
				}
				const originalSet = readFileSync(join(directory.path, 'set.json'), 'utf8');
				const originalRules = readFileSync(join(directory.path, 'rules.jsonl'), 'utf8');
				const wrong = await loadRuleSet({
					path: 'set.json',
					profile,
					inputContract: {
						subject: { kind: sourceKind === 'document' ? 'conversation' : 'document' },
					},
				});
				assert.equal(wrong.kind, 'invalid');
				assert.equal(wrong.diagnostics.items[0]?.code, 'input_kind_mismatch');
				const loaded = await loadRuleSet({
					path: 'set.json',
					inputContract: { subject: { kind: sourceKind } },
					profile,
				});
				assert.equal(loaded.kind, 'ready');
				writeFiles(directory.path, { 'set.json': 'not JSON', 'rules.jsonl': 'not JSON' });
				const invocation = { inputs: { subject }, origin: input.origin };
				const retained = await prepareRules(loaded.ruleSet, invocation);
				assert.equal(retained.kind, 'ready_single');
				assert.equal(retained.request.body, originalPreview.request.body);
				assert.equal(backend.received.length, 0);

				process.env['OPENROUTER_API_KEY'] = 'synthetic-general-rules-test';
				const cancelledEarly = await evaluateRules(loaded.ruleSet, invocation, {
					signal: AbortSignal.abort(),
				});
				assert.equal(cancelledEarly.kind, 'cancelled');
				assert.equal(cancelledEarly.progress.kind, 'before_report_admission');
				assert.equal(backend.received.length, 0);
				const result = await evaluateRules(loaded.ruleSet, invocation);

				assert.equal(
					result.kind,
					'complete_report',
					JSON.stringify(result, (_key, value: unknown) =>
						typeof value === 'bigint' ? value.toString() : value,
					),
				);
				assert.equal(result.report.occurrences[0]?.kind, 'judged');
				assert.equal(result.report.measurements.model_requests, 1n);
				assert.equal(backend.received.length, 1);
				const request = backend.received[0];
				assert.ok(request);
				assert.equal(request.rawBody.toString('utf8'), retained.request.body);
				assert.match(request.rawBody.toString('utf8'), new RegExp(fixture.sentinel, 'u'));

				if (sourceKind === 'document') {
					assert.equal(evidence.content.kind, 'matches');
					const selected = evidence.content.matches[0];
					assert.ok(selected);
					assert.equal(selected.text, 'console.log("DOCUMENT_SENTINEL")');
					assert.equal(selected.span.byte_start, 0n);
					assert.equal(selected.span.byte_end, 32n);
					const state = record(request.body['state']);
					const astEntry = Object.entries(record(state['evidence'])).find(([key]) =>
						key.endsWith('/ast_matches'),
					);
					assert.ok(astEntry);
					const entry = record(astEntry[1]);
					assert.equal(entry['kind'], 'matches');
					const contentId = entry['content'];
					assert.ok(typeof contentId === 'string');
					const contents = record(state['contents'])[contentId];
					assert.ok(Array.isArray(contents));
					assert.equal(contents.length, 1);
					const modelMatch = record(contents[0]);
					assert.equal(modelMatch['text'], selected.text);
					const span = record(modelMatch['span']);
					assert.equal(span['byte_start'], Number(selected.span.byte_start));
					assert.equal(span['byte_end'], Number(selected.span.byte_end));
					assert.deepEqual(span['source'], selected.span.source);
				}

				const saved = serialize({ kind: 'rule_invocation_outcome', value: result });
				assert.equal(saved.kind, 'serialized');
				if (matchingCli !== undefined) {
					writeFiles(directory.path, { 'set.json': originalSet, 'rules.jsonl': originalRules });
					const cli = await callMatchingCli(matchingCli, input);
					const sdkReport = record(record(record(JSON.parse(saved.json))['payload'])['value'])[
						'report'
					];
					const cliReport = record(cli['report']);
					const decoded = record(sdkReport);
					assert.equal(cli['kind'], 'complete_report');
					for (const field of [
						'captures',
						'occurrences',
						'rule_evaluations',
						'groups',
						'cursors',
					] as const) {
						assert.deepEqual(decoded[field], cliReport[field], field);
					}
					assert.equal(backend.received.length, 2);
					assert.deepEqual(backend.received[1]?.rawBody, request.rawBody);
					context.diagnostic(
						'matching CLI returned the same native evidence and occurrences and sent identical request bytes',
					);
				}
				if (sourceKind === 'conversation') {
					writeFiles(directory.path, { 'set.json': originalSet, 'rules.jsonl': originalRules });
					await checkPendingEndings(input);
				}
				const restored = await deserialize(saved.json);
				assert.deepEqual(restored, {
					kind: 'decoded',
					value: { kind: 'rule_invocation_outcome', value: result },
				});
				if (sourceKind === 'document') {
					const largeValue = {
						...result,
						report: {
							...result.report,
							measurements: {
								...result.report.measurements,
								provider_elapsed_ms: 9_007_199_254_740_993n,
							},
						},
					} as const;
					const exact = serialize({ kind: 'rule_invocation_outcome', value: largeValue });
					assert.equal(exact.kind, 'serialized');
					assert.match(exact.json, /"provider_elapsed_ms":9007199254740993/u);
					assert.deepEqual(await deserialize(exact.json), {
						kind: 'decoded',
						value: { kind: 'rule_invocation_outcome', value: largeValue },
					});
					const tampered = exact.json.replace('9007199254740993', '"9007199254740993"');
					assert.equal((await deserialize(tampered)).kind, 'invalid');
					const oldRevision = exact.json.replace(
						/"contract_revision":\d+/u,
						'"contract_revision":22',
					);
					assert.equal((await deserialize(oldRevision)).kind, 'incompatible');
				}

				const savedEvidence = serialize({ kind: 'evidence_plan_outcome', value: inspected });
				assert.equal(savedEvidence.kind, 'serialized');
				assert.deepEqual(await deserialize(savedEvidence.json), {
					kind: 'decoded',
					value: { kind: 'evidence_plan_outcome', value: inspected },
				});
				context.diagnostic(
					`one ${sourceKind} occurrence judged; captured request equals the keyless preview after rule file replacement`,
				);
			} finally {
				await responder.stop();
				await backend.close();
				directory.remove();
			}
		},
	);
}

async function checkOtherPreviewStates(
	input: RuleInvocationInput,
	root: string,
	subject: InputBinding,
): Promise<void> {
	const originalSet = readFileSync(join(root, 'set.json'), 'utf8');
	const set = record(JSON.parse(originalSet));
	try {
		const multi = await prepareRules({
			...input,
			definitions: {
				kind: 'rule_sets',
				rule_sets: [
					{
						path: 'set.json',
						binding_groups: [{ bindings: { subject } }, { bindings: { subject } }],
					},
				],
			},
		});
		assert.equal(multi.kind, 'unsupported_multiple');
		assert.deepEqual(
			multi.ready.map((request) => request.index),
			[0n, 1n],
		);
		writeFiles(root, {
			'set.json': JSON.stringify({
				...set,
				applicability: [
					{
						kind: 'condition',
						test: {
							kind: 'pattern_present',
							input: 'subject',
							pattern: { kind: 'literal', text: 'not in this fixture' },
							max_scan_bytes: 4096,
						},
					},
				],
			}),
		});
		const mechanical = await prepareRules(input);
		assert.equal(mechanical.kind, 'no_model_work');
		assert.equal(mechanical.report.groups[0]?.applicability.kind, 'not_applicable');
		assert.equal(mechanical.report.occurrences.length, 0);
		assert.equal(mechanical.report.measurements.model_requests, 0n);

		writeFiles(root, {
			'gate.md': 'U001: separate gate source\n',
			'set.json': JSON.stringify({
				...set,
				inputs: { subject: { kind: 'document' }, gate: { kind: 'document' } },
				applicability: [
					{
						kind: 'decision',
						question: { type: 'noul', instructions: 'Gate has separate evidence' },
						evidence: [
							{ id: 'gate', input: 'gate', select: { kind: 'paragraph', target_id: 'U001' } },
						],
						threshold: 0.6,
					},
				],
			}),
		});
		const pending = await prepareRules({
			...input,
			definitions: {
				kind: 'rule_sets',
				rule_sets: [
					{
						path: 'set.json',
						binding_groups: [
							{
								bindings: {
									subject,
									gate: { kind: 'document', repository: 'fixture', path: 'gate.md' },
								},
							},
						],
					},
				],
			},
			repositories: input.repositories.map((repository) => ({
				...repository,
				input_paths: [...repository.input_paths, { kind: 'exact', path: 'gate.md' }],
			})),
		});
		assert.equal(pending.kind, 'pending_dependency');
		assert.equal(pending.ready.length, 1);
		assert.match(pending.ready[0]?.body ?? '', /Gate has separate evidence/u);
		assert.doesNotMatch(pending.ready[0]?.body ?? '', /Check selected input/u);
	} finally {
		writeFiles(root, { 'set.json': originalSet });
	}
}

async function callMatchingCli(
	binary: string,
	input: RuleInvocationInput,
): Promise<Readonly<Record<string, unknown>>> {
	const wire = encodeForNative({ protocol_version: 1, operation: 'evaluate_rules', input });
	assert.equal(wire.kind, 'encoded');
	const process = spawn(binary, ['exec'], {
		cwd: input.definition_root,
		stdio: ['pipe', 'pipe', 'pipe'],
	});
	let stdout = '';
	let stderr = '';
	process.stdout.on('data', (chunk: Buffer) => {
		stdout += chunk.toString();
	});
	process.stderr.on('data', (chunk: Buffer) => {
		stderr += chunk.toString();
	});
	const exit = new Promise<number | null>((resolve, reject) => {
		process.once('error', reject);
		process.once('close', resolve);
	});
	process.stdin.end(wire.json);
	assert.equal(await exit, 0, stderr);
	const reply = record(JSON.parse(stdout));
	assert.equal(reply['kind'], 'operation_result');
	return record(record(reply['result'])['outcome']);
}

async function checkPendingEndings(input: RuleInvocationInput): Promise<void> {
	await checkPendingEnding(input, 'abort');
	await checkPendingEnding(input, 'drop');
}

async function checkPendingEnding(
	input: RuleInvocationInput,
	ending: 'abort' | 'drop',
): Promise<void> {
	const backend = await startControlledBackend();
	const abort = new AbortController();
	try {
		const run = evaluateRules(
			{
				...input,
				backend: { ...input.backend, endpoint: backend.endpoint },
			},
			{ signal: abort.signal },
		);
		await Promise.race([
			backend.nextRequest(),
			run.then((result) => {
				throw new Error(`Evaluation ended before dispatch: ${result.kind}.`);
			}),
		]);
		if (ending === 'abort') abort.abort();
		else await backend.close();
		const result = await run;
		if (ending === 'abort') {
			assert.equal(result.kind, 'cancelled');
			assert.equal(result.progress.kind, 'report_admitted');
			const request = result.progress.partial_report.requests[0]?.status;
			assert.equal(request?.kind, 'sent');
			assert.equal(request.outcome.kind, 'cancelled');
			assert.equal(request.outcome.attempts[0]?.evidence.kind, 'remote_outcome_unknown');
		} else {
			assert.equal(result.kind, 'incomplete_report');
			const request = result.report.requests[0]?.status;
			assert.equal(request?.kind, 'sent');
			assert.equal(request.outcome.kind, 'provider_failure');
			assert.equal(request.outcome.attempts.length, 1);
			assert.equal(request.outcome.attempts[0]?.evidence.kind, 'remote_outcome_unknown');
		}
		assert.equal(backend.received.length, 1);
	} finally {
		if (ending === 'abort') await backend.close();
	}
}
