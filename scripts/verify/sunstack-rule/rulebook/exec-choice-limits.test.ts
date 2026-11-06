import { expect, test } from 'vitest';

import { execChoiceInput, requireExecChoiceOutcome } from './exec-choice-limits.ts';
import { arrayValue, objectValue, type JsonObject } from './rulebook-fixture-support.ts';
import { responseBody, type CapturedRequest } from './rulebook-process.ts';

function completed(): { report: JsonObject; captures: CapturedRequest[]; count: 255 } {
	const request = objectValue(objectValue(execChoiceInput(255, 'unused')['input'])['request']);
	const body: JsonObject = { model: 'typesafe/exec-choice', ...request };
	const remote = responseBody(body, 1, 'option_000');
	const answer = objectValue(objectValue(remote['answers'])['classification']);
	return {
		count: 255,
		captures: [{ body, raw: Buffer.from(JSON.stringify(body)) }],
		report: {
			protocol_version: 1,
			kind: 'operation_result',
			result: {
				operation: 'run_judgments',
				outcome: {
					kind: 'completed',
					answers: {
						classification: {
							type: 'choice',
							selected_label: 'option_000',
							probabilities: answer['probabilities'] ?? null,
						},
					},
					receipt: { resolved_model: 'typesafe/controlled-rulebook' },
					attempts: [{}],
				},
			},
		},
	};
}
function refused(): { report: JsonObject; captures: CapturedRequest[]; count: 256 } {
	return {
		count: 256,
		captures: [],
		report: {
			protocol_version: 1,
			kind: 'operation_result',
			result: {
				operation: 'run_judgments',
				outcome: {
					kind: 'invalid_request',
					diagnostics: {
						items: [
							{
								origin: 'request',
								code: 'invalid_choice',
								retry_advice: { kind: 'repair_request' },
								guidance: 'Choice has 256 options; maximum is 255.',
							},
						],
						omitted_count: 0,
					},
				},
			},
		},
	};
}
test('exec checker accepts real completion and a distinct local cap outcome', () => {
	expect(() => requireExecChoiceOutcome(completed())).not.toThrow();
	expect(() => requireExecChoiceOutcome(refused())).not.toThrow();
});
test('positive control rejects zero dispatch, missing option and incomplete completion', () => {
	const noDispatch = completed();
	noDispatch.captures = [];
	expect(() => requireExecChoiceOutcome(noDispatch)).toThrow();
	const truncated = completed();
	delete objectValue(
		objectValue(objectValue(truncated.captures[0]?.body['questions'])['classification'])[
			'criteria'
		],
	)['option_254'];
	expect(() => requireExecChoiceOutcome(truncated)).toThrow();
	const incomplete = completed();
	objectValue(objectValue(incomplete.report['result'])['outcome'])['kind'] = 'invalid_response';
	expect(() => requireExecChoiceOutcome(incomplete)).toThrow();
});
test.each([
	['origin', 'response'],
	['code', 'invalid_configuration'],
	['retry_advice', 'repair_request'],
	['retry_advice', 'do_not_retry'],
	['guidance', 'Repair request.'],
	['guidance', 'Choice has 255 options; maximum is 255.'],
	['guidance', 'Choice has 256 options; maximum is 999.'],
] as const)('cap checker rejects wrong diagnostic %s=%s', (field, value) => {
	const result = refused();
	const diagnostics = objectValue(
		objectValue(objectValue(result.report['result'])['outcome'])['diagnostics'],
	);
	objectValue(arrayValue(diagnostics['items'])[0])[field] = value;
	expect(() => requireExecChoiceOutcome(result)).toThrow();
});
test('cap checker rejects protocol errors, provider refusal and hidden dispatch', () => {
	const protocol = refused();
	protocol.report['kind'] = 'protocol_error';
	expect(() => requireExecChoiceOutcome(protocol)).toThrow();
	const provider = refused();
	objectValue(objectValue(provider.report['result'])['outcome'])['kind'] = 'provider_failure';
	expect(() => requireExecChoiceOutcome(provider)).toThrow();
	const sent = refused();
	sent.captures = completed().captures;
	expect(() => requireExecChoiceOutcome(sent)).toThrow();
	const attempted = refused();
	objectValue(objectValue(attempted.report['result'])['outcome'])['attempts'] = [{}];
	expect(() => requireExecChoiceOutcome(attempted)).toThrow();
});
