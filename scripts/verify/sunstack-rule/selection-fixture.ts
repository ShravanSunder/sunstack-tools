/** Copies the Implementer's pinned S0 selection fixture; never edits the product fixture. */
import filesystem from 'node:fs';
import path from 'node:path';

import {
	arrayValue,
	configName,
	configure,
	objectValue,
	readObject,
	requireCondition,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';
export const selectionRelative = 'crates/sunstack/tests/fixtures/rulebook-lint/selection';
export const researchFile = 'skills/practices-research/SKILL.md';
export const presentationFile = 'skills/presentation-tui/SKILL.md';
export const emptyFile = 'skills/no-list/SKILL.md';
export const manyFile = 'skills/thirty-three/SKILL.md';
export function selectionKinds(root: string): JsonObject {
	return objectValue(objectValue(readObject(path.join(root, configName))['evidence'])['kinds']);
}
export function changeSelectionKinds(root: string, change: (kinds: JsonObject) => void): void {
	configure(root, (config) => {
		change(objectValue(objectValue(config['evidence'])['kinds']));
	});
}
export function copySelectionFixture(options: {
	destination: string;
	sourceRoot: string;
	files?: readonly string[];
	allKinds?: boolean;
}): string {
	requireCondition(
		!filesystem.existsSync(options.destination),
		'Selection destination must be fresh',
	);
	filesystem.cpSync(path.join(options.sourceRoot, selectionRelative), options.destination, {
		recursive: true,
	});
	configure(options.destination, (config) => {
		config['check'] = {
			files: [...(options.files ?? [researchFile, presentationFile, emptyFile])],
		};
		if (!options.allKinds) {
			const evidence = objectValue(config['evidence']);
			const kinds = objectValue(evidence['kinds']);
			evidence['kinds'] = Object.fromEntries(
				['workflow_section', 'step', 'load'].map((kind) => {
					const value = kinds[kind];
					requireCondition(value, 'S0 kind missing');
					return [kind, value];
				}),
			);
		}
		config['decision_model'] = { model: 'jev-latest', endpoint: 'http://127.0.0.1:9/systemone' };
		config['limits'] = {
			max_model_requests: 200,
			max_concurrent_requests: 1,
			request_max_chars: 100000,
		};
	});
	return options.destination;
}
export function oracleRows(root: string, rule: string, file: string): JsonObject[] {
	return arrayValue(readObject(path.join(root, 'scan-oracle.json'))['matches'])
		.map(objectValue)
		.filter((row) => row['rule'] === rule && row['file'] === file);
}
export function selectedQuestion(root: string, kind = 'workflow_section'): JsonObject {
	return objectValue(objectValue(objectValue(selectionKinds(root)[kind])['select'])['question']);
}
export function writeChecked(root: string, file: string, text: string): void {
	const destination = path.join(root, file);
	filesystem.mkdirSync(path.dirname(destination), { recursive: true });
	filesystem.writeFileSync(destination, text);
}
export function keepKinds(root: string, names: readonly string[]): void {
	configure(root, (config) => {
		const evidence = objectValue(config['evidence']);
		const kinds = objectValue(evidence['kinds']);
		evidence['kinds'] = Object.fromEntries(
			names.map((name) => {
				const value = kinds[name];
				requireCondition(value, 'Missing kind ' + name);
				return [name, value];
			}),
		);
	});
	if (!names.includes('step')) {
		// Evidence-only probes still admit their rulebook. Keep the fixture's
		// mechanical body witness valid when its original step kind is removed.
		const rules = readObject(path.join(root, 'rules.json'));
		const entry = objectValue(rules['K001']);
		requireCondition(entry['evidence'] === 'step', 'Expected the canonical S0 step-body witness');
		const kinds = selectionKinds(root);
		const bodyKind = names.find(
			(name) => 'body' in objectValue(objectValue(kinds[name])['fields']),
		);
		requireCondition(bodyKind, 'Pruned fixture needs an explicit valid rule witness');
		entry['evidence'] = bodyKind;
		writeJson(path.join(root, 'rules.json'), rules);
	}
}
export function askingStepRule(root: string): void {
	writeJson(path.join(root, 'rules.json'), {
		K001: {
			name: 'selected_step_asks',
			evidence: 'step',
			severity: 'warn',
			questions: {
				verdict: {
					type: 'noul',
					instructions: { check: 'body', task: 'Does this step meet the controlled standard?' },
				},
			},
			decision: {
				start: 'verdict',
				steps: {
					verdict: {
						ask: 'verdict',
						yes_at_or_above: 0.8,
						no_at_or_below: 0.3,
						yes: 'pass',
						['no']: { finding: 'Controlled finding.' },
					},
				},
			},
		},
	});
}
