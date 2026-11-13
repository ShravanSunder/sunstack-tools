/** Candidate/shared cut combinations and authored-marker negative, all through real CLI prepare. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import { writeProject } from '../html-evidence/html-fixtures.ts';
import {
	arrayValue,
	numberValue,
	objectValue,
	textValue,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';

export const cases = [
	{ name: 'literal-uncut', candidateCut: false, sharedCut: false, list: false },
	{ name: 'candidate-cut', candidateCut: true, sharedCut: false, list: false },
	{ name: 'shared-cut', candidateCut: false, sharedCut: true, list: false },
	{ name: 'both-cut', candidateCut: true, sharedCut: true, list: false },
	{ name: 'shared-list-cut', candidateCut: false, sharedCut: true, list: true },
] as const;
export type ControlCase = (typeof cases)[number];
export function prepareControls(common: string): void {
	for (const control of cases) {
		const root = path.join(common, control.name);
		filesystem.mkdirSync(root, { recursive: true });
		const candidate = control.candidateCut
			? 'Candidate: '.concat('body '.repeat(1000))
			: 'Candidate: authored [cut] stays literal.';
		const opening = control.list
			? Array.from({ length: 100 }, (_, index) => `Opening: item ${index} ${'x'.repeat(20)}`).join(
					'\n\n',
				)
			: control.sharedCut
				? 'Opening: '.concat('context '.repeat(1000))
				: 'Opening: authored [cut: 7 more] stays literal.';
		filesystem.writeFileSync(path.join(root, 'source.md'), `${candidate}\n\n${opening}\n`);
		writeProject(
			root,
			{
				K001: {
					name: 'cut-state',
					evidence: 'candidate',
					severity: 'warn',
					decision: {
						start: 'check',
						steps: {
							check: {
								mechanical_check: { tool: 'complete', field: 'body' },
								yes: 'pass',
								['no']: 'pass',
							},
						},
					},
				},
			},
			{
				candidate: {
					rule: 'candidate',
					max_chars: 1000,
					fields: {
						body: { from: 'remainder' },
						extra: { from: 'remainder' },
						opening: { from: 'same_file', rule: 'opening', text: 'full' },
					},
					select: {
						question: {
							type: 'noul',
							instructions: {
								check: 'body',
								context: ['extra', 'opening'],
								task: 'Inspect the unchanged text with truthful cut names.',
							},
						},
						yes_at_or_above: 0.7,
						no_at_or_below: 0.3,
						shared: ['opening'],
					},
				},
			},
			['source.md'],
			'id: candidate\nlanguage: Markdown\nrule: {kind: paragraph, regex: "^Candidate:"}\n---\nid: opening\nlanguage: Markdown\nrule: {kind: paragraph, regex: "^Opening:"}\n',
		);
	}
}
export function requireControlCounts(
	result: JsonObject,
	control: ControlCase,
	before: JsonObject,
): void {
	const requests = arrayValue(before['requests']);
	assert.equal(requests.length, 1);
	const evidence = objectValue(objectValue(requests[0])['selection_evidence']);
	const candidates = arrayValue(evidence['candidates']);
	assert.equal(candidates.length, 1);
	const view = objectValue(objectValue(candidates[0])['selection_view']);
	const shared = objectValue(objectValue(evidence['shared'])['view']);
	assert.deepEqual(view['cut'], control.candidateCut ? ['body', 'extra'] : []);
	assert.deepEqual(shared['cut'], control.sharedCut ? ['opening'] : []);
	if (control.list)
		assert.match(
			textValue(arrayValue(objectValue(shared['fields'])['opening']).at(-1)),
			/^\[cut: [1-9]\d* more\]$/u,
		);
	assert.equal(numberValue(result['candidate_cut']), control.candidateCut ? 1 : 0);
	assert.equal(numberValue(result['candidate_uncut']), control.candidateCut ? 0 : 1);
	assert.equal(numberValue(result['shared_cut']), control.sharedCut ? 1 : 0);
	assert.equal(numberValue(result['shared_uncut']), control.sharedCut ? 0 : 1);
	assert.equal(
		numberValue(result['changed_hashes']),
		control.candidateCut || control.sharedCut ? 1 : 0,
	);
	assert.equal(
		numberValue(result['unchanged_hashes']),
		control.candidateCut || control.sharedCut ? 0 : 1,
	);
}
