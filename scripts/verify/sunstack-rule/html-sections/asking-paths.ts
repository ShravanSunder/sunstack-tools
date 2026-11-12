/** C5: selection/shared and filter preparation cross the same HTML asking boundary as rules. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import { writeProject } from '../html-evidence/html-fixtures.ts';
import {
	arrayValue,
	objectValue,
	numberValue,
	readObject,
	writeJson,
	runSequence,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import type { RulebookProcess } from '../rulebook/rulebook-process.ts';
import { askingRule, askingPathHtml, askingPathText } from './fixtures.ts';

function preparedRequest(report: JsonObject, owner: string): JsonObject {
	assert.equal(objectValue(report['usage'])['model_requests'], 0);
	const records = arrayValue(report['requests']).map(objectValue);
	assert.equal(records.length, 1, 'Asking-path fixture must prepare exactly its intended request');
	const record = records[0];
	assert.ok(record);
	assert.equal(objectValue(record['owner'])['kind'], owner);
	assert.equal(record['status'], 'prepared');
	return record;
}
export function requireSelectionProjection(report: JsonObject): void {
	const record = preparedRequest(report, 'selection');
	const state = objectValue(objectValue(record['request'])['state']);
	assert.deepEqual(
		state['candidates'],
		{ ['c1']: { full: askingPathText } },
		'Selection candidates still carry raw HTML or lose meaning',
	);
	assert.deepEqual(
		state['shared'],
		{ context: [askingPathText] },
		'Shared selection view bypassed HTML projection',
	);
	const evidence = objectValue(record['selection_evidence']);
	const candidates = arrayValue(evidence['candidates']).map(objectValue);
	assert.equal(candidates.length, 1);
	const candidate = candidates[0];
	assert.ok(candidate);
	assert.equal(
		objectValue(objectValue(candidate['fields'])['full'])['value'],
		askingPathHtml,
		'Selection mutated original raw fields',
	);
	assert.deepEqual(
		objectValue(objectValue(objectValue(evidence['shared'])['fields'])['context'])['values'],
		[askingPathHtml],
	);
}
export function requireFilterProjection(report: JsonObject): void {
	const record = preparedRequest(report, 'filter');
	const state = objectValue(objectValue(record['request'])['state']);
	assert.deepEqual(
		state,
		{ full: askingPathText, 'unavailable-fields': {} },
		'Filter request bypassed the HTML projection',
	);
	const piece = objectValue(
		objectValue(objectValue(report['piece_evidence'])[String(numberValue(record['id']))])['piece'],
	);
	assert.equal(piece['text'], askingPathHtml);
	assert.equal(
		objectValue(objectValue(piece['fields'])['full'])['value'],
		askingPathHtml,
		'Filter mutated raw declared reads',
	);
}
export function prepareAskingPath(root: string, selection: boolean): void {
	filesystem.mkdirSync(root);
	const directory = selection ? root : path.join(root, 'groups/one');
	filesystem.mkdirSync(directory, { recursive: true });
	filesystem.writeFileSync(
		path.join(directory, selection ? 'projection.html' : 'overview.html'),
		askingPathHtml,
	);
	if (!selection)
		filesystem.writeFileSync(path.join(directory, 'subject.html'), '<p>Checked member.</p>');
	const rule: JsonObject = {
		...askingRule,
		...(!selection ? { only_when: { category: ['eligible'] } } : {}),
		questions: {
			quality: { type: 'noul', instructions: { check: 'full', task: 'Inspect the picture text.' } },
		},
	};
	writeProject(
		root,
		{ K001: rule },
		{
			section: {
				rule: 'whole',
				max_chars: 1000,
				fields: {
					full: { from: 'remainder' },
					...(selection ? { context: { from: 'same_file', rule: 'whole', text: 'full' } } : {}),
				},
				...(selection
					? {
							select: {
								question: {
									type: 'noul',
									instructions: {
										check: 'full',
										context: ['context'],
										task: 'Keep this HTML evidence?',
									},
								},
								yes_at_or_above: 0.7,
								no_at_or_below: 0.3,
								shared: ['context'],
							},
						}
					: {}),
			},
		},
		[selection ? 'projection.html' : 'groups/*/subject.html'],
		'id: whole\nlanguage: Html\nrule: {kind: document}\n',
	);
	if (!selection) {
		const configFile = path.join(root, 'sunstack.lint.json');
		const config = readObject(configFile);
		config['filters'] = {
			category: {
				per: 'groups/{name}',
				reads: { file: 'overview.html', kind: 'section', fields: ['full'] },
				labels: ['eligible', 'blocked'],
				questions: {
					purpose: {
						type: 'choice',
						instructions: { check: 'full', task: 'Classify the picture text.' },
						criteria: { eligible: 'Relevant.', blocked: 'Unrelated.' },
					},
				},
				decision: {
					start: 'purpose',
					steps: {
						purpose: {
							ask: 'purpose',
							min_probability: 0.7,
							routes: { eligible: { label: 'eligible' }, blocked: { label: 'blocked' } },
							uncertain: 'unsure',
						},
					},
				},
			},
		};
		writeJson(configFile, config);
	}
}
export async function observeOtherAskingPaths(options: {
	observer: RulebookProcess;
	evidence: string;
}): Promise<void> {
	await runSequence(['selection', 'filter'], async (kind) => {
		const root = path.join(options.evidence, `${kind}-asking`);
		prepareAskingPath(root, kind === 'selection');
		const configFile = path.join(root, 'sunstack.lint.json');
		const config = readObject(configFile);
		objectValue(config['decision_model'])['endpoint'] = options.observer.endpoint;
		writeJson(configFile, config);
		await options.observer.run({
			root,
			name: `${kind}-asking-check`,
			arguments: ['lint', '--check'],
			expectedExit: 0,
		});
		const file = path.join(options.evidence, `${kind}-asking.json`);
		await options.observer.run({
			root,
			name: `${kind}-asking-prepare`,
			arguments: ['lint', '--all', '--prepare', file],
			expectedExit: 6,
		});
		const report = readObject(file);
		if (kind === 'selection') requireSelectionProjection(report);
		else requireFilterProjection(report);
	});
}
