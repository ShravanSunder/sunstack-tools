/** Scripted native answers keyed by file and candidate, never inferred from product outcomes. */
import { configName, objectValue, type JsonObject } from './rulebook-fixture-support.ts';
import { responseBody } from './rulebook-process.ts';
import { selectionState } from './selection-checks.ts';
export interface SelectionReply {
	probability: (file: string, key: string) => number;
	ruleProbability?: number;
}
export function scriptedSelectionReply(options: SelectionReply): (body: JsonObject) => JsonObject {
	return (body) => {
		const state = objectValue(body['state']);
		if (!('candidates' in state)) return responseBody(body, options.ruleProbability ?? 0.91);
		const selected = selectionState(body);
		const answers: JsonObject = {};
		for (const key of Object.keys(objectValue(body['questions'])))
			answers[key] = { type: 'noul', noul: options.probability(selected.file, key) };
		return {
			model: 'typesafe/controlled-selection',
			answers,
			usage: { input_tokens: 20, output_tokens: 5, cost: 0.000001 },
		};
	};
}

export function selectionArguments(...extra: string[]): string[] {
	return ['lint', '--config', configName, '--format', 'json', ...extra];
}
