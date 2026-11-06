/** Real exec RunJudgments boundary:255 options complete over HTTP;256 refuse locally. */
import filesystem from 'node:fs';
import path from 'node:path';

import { requireDebugSignature } from './rulebook-amendment.ts';
import {
	arrayValue,
	equal,
	errorMessage,
	journeyArguments,
	objectValue,
	requireCondition,
	runSequence,
	textValue,
	writeJson,
	type JsonObject,
	type JourneyArguments,
} from './rulebook-fixture-support.ts';
import {
	declaredTarget,
	responseBody,
	RulebookProcess,
	type CapturedRequest,
} from './rulebook-process.ts';

const model = 'typesafe/exec-choice';
const state = { description: 'Bounded Choice protocol witness.', sentinel: 'keep-☃-escaped"text' };
export function execChoiceInput(count: number, endpoint: string): JsonObject {
	return {
		protocol_version: 1,
		operation: 'run_judgments',
		input: {
			request: {
				questions: {
					classification: {
						type: 'choice',
						instructions: { task: 'Choose the applicable option.' },
						criteria: Object.fromEntries(
							Array.from({ length: count }, (_value, index) => [
								`option_${String(index).padStart(3, '0')}`,
								`Criterion ${index}.`,
							]),
						),
					},
				},
				state,
			},
			backend: { kind: 'jev_openrouter', model, endpoint },
			attempts: {
				attempt_timeout_ms: 10000,
				retry_count: 0,
				initial_backoff_ms: 100,
				max_backoff_ms: 1000,
				max_response_bytes: 1048576,
			},
			request_budget: {
				max_input_tokens: 100000,
				max_request_bytes: 100000,
				max_questions: 10,
				max_requests: 10,
				safety_margin_tokens: 32,
				accounting_policy: 'allow_estimated',
			},
		},
	};
}

export function requireExecChoiceOutcome(options: {
	report: JsonObject;
	captures: readonly CapturedRequest[];
	count: 255 | 256;
}): void {
	const { report, captures, count } = options;
	equal(report['protocol_version'], 1, 'Wrong protocol version');
	equal(report['kind'], 'operation_result', 'Protocol parsing error is not a cap refusal');
	const result = objectValue(report['result']);
	equal(result['operation'], 'run_judgments', 'Wrong exec operation');
	const outcome = objectValue(result['outcome']);
	if (count === 255) {
		equal(captures.length, 1, '255 must actually dispatch exactly once');
		equal(outcome['kind'], 'completed', '255 must complete, not merely dispatch');
		const request = objectValue(objectValue(execChoiceInput(255, 'unused')['input'])['request']);
		const capture = captures[0];
		requireCondition(capture, 'Positive HTTP witness missing');
		equal(
			capture.body,
			{ model, state, questions: request['questions'] },
			'Actual provider body changed',
		);
		const answer = objectValue(objectValue(outcome['answers'])['classification']);
		equal(answer['type'], 'choice', 'Choice answer lost');
		equal(answer['selected_label'], 'option_000', 'Wrong scripted answer returned');
		equal(
			answer['probabilities'],
			Object.fromEntries(
				Array.from({ length: 255 }, (_value, index) => [
					`option_${String(index).padStart(3, '0')}`,
					index === 0 ? 1 : 0,
				]),
			),
			'Answer probabilities were truncated',
		);
		equal(
			objectValue(outcome['receipt'])['resolved_model'],
			'typesafe/controlled-rulebook',
			'Wrong completion receipt',
		);
		equal(arrayValue(outcome['attempts']).length, 1, 'Wrong positive attempt count');
	} else {
		equal(captures.length, 0, '256 reached provider HTTP');
		equal(
			outcome['kind'],
			'invalid_request',
			'Provider failure/body mismatch is not local cap validation',
		);
		requireCondition(
			!('attempts' in outcome) && !('answers' in outcome),
			'Refused request fabricated dispatch or answers',
		);
		const diagnostics = objectValue(outcome['diagnostics']);
		equal(diagnostics['omitted_count'], 0, 'Cap diagnostic was omitted');
		const items = arrayValue(diagnostics['items']).map(objectValue);
		requireCondition(items.length > 0, 'No actionable local diagnostic');
		requireCondition(
			items.some(
				(diagnostic) =>
					diagnostic['origin'] === 'request' &&
					diagnostic['code'] === 'invalid_choice' &&
					objectValue(diagnostic['retry_advice'])['kind'] === 'repair_request' &&
					/\b256\b/.test(textValue(diagnostic['guidance'])) &&
					/\b255\b/.test(textValue(diagnostic['guidance'])),
			),
			'Local Choice diagnostic lost actual count/limit',
		);
	}
}

export async function runExecChoiceJourney(options: JourneyArguments): Promise<void> {
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const fixture = path.join(evidence, 'fixture');
	filesystem.mkdirSync(fixture);
	for (const count of [255, 256])
		writeJson(
			path.join(fixture, `input-${count}.json`),
			execChoiceInput(count, 'http://127.0.0.1:1/systemone'),
		);
	if (options.prepare) {
		writeJson(path.join(evidence, 'prepared.json'), {
			product_observed: false,
			cases: [255, 256],
			oracle: 'Lead correction disposition01a1aefd-fd5e/F1',
		});
		process.stdout.write('Prepared exec Choice fixtures; product unobserved\n');
		return;
	}
	const target = declaredTarget({
		binary: options.binary ?? '',
		source: options['source-root'],
		commit: options.commit ?? '',
		...(options.sha256 ? { digest: options.sha256 } : {}),
	});
	requireDebugSignature(target.binary);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	observer.reply = (body) => responseBody(body, 1, 'option_000');
	try {
		await runSequence([255, 256] as const, async (count) => {
			const input = execChoiceInput(count, observer.endpoint);
			writeJson(path.join(fixture, `input-${count}.json`), input);
			const result = await observer.report({
				root: fixture,
				name: `choice-${count}`,
				arguments: ['exec'],
				input: Buffer.from(JSON.stringify(input) + '\n'),
				expectedExit: count === 255 ? 0 : 2,
				credential: true,
			});
			requireExecChoiceOutcome({ ...result, count });
		});
	} finally {
		await observer.close();
	}
	writeJson(path.join(evidence, 'pass.json'), {
		commit: target.commit,
		binary_sha256: target.digest,
		oracle: 'Lead correction disposition01a1aefd-fd5e/F1; provider Choice max255',
		observed: [
			'real exec255 completed with exact provider body/answer/receipt',
			'exec256 local invalid_choice with count256/limit255 and zeroHTTP',
		],
		required_native_joins: [
			'run_judgments',
			'run_admitted_judgments with independently admitted bytes',
			'F2 lazy mechanical outcomes',
			'F3 admission and runtime typed filter diagnostic',
		],
		unverified: ['hosted inference quality', 'CI/install/release'],
	});
	process.stdout.write(
		'Exec Choice255/256 journey passed; direct API/filter/native joins remain separate\n',
	);
}

if (import.meta.main)
	try {
		const options = journeyArguments(
			'Exec Choice cap:255 reaches real HTTP/completes;256 returns local count/limit with zeroHTTP.',
		);
		if (options) await runExecChoiceJourney(options);
	} catch (error) {
		process.stderr.write(`Exec Choice observation failed: ${errorMessage(error)}\n`);
		process.exitCode = 2;
	}
