/** Immutable saved-report inputs and an independent inventory of their result owners. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import {
	arrayValue,
	numberValue,
	objectValue,
	parseObject,
	textValue,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';

export interface SavedCandidate {
	readonly revision: string;
	readonly caseIds: string[];
	readonly report: JsonObject;
	readonly result: JsonObject;
}
export function candidatesFor(report: JsonObject, id: number): SavedCandidate[] {
	const cases = arrayValue(report['cases']).map(objectValue);
	return arrayValue(report['runs'])
		.map(objectValue)
		.flatMap((run) => {
			const revision = textValue(run['revision']);
			const saved = objectValue(run['report']);
			const rows = ['results', 'filter_answers', 'filter_skips'].flatMap((bucket) =>
				arrayValue(saved[bucket]).map(objectValue),
			);
			return rows
				.filter((row) => row['id'] === id)
				.map((result) => ({
					revision,
					report: saved,
					result,
					caseIds: cases
						.filter(
							(entry) =>
								objectValue(entry['case'])['revision'] === revision && entry['result_id'] === id,
						)
						.map((entry) => textValue(entry['id'])),
				}));
		});
}
export function candidateForCase(report: JsonObject, caseId: string): SavedCandidate {
	const selected = arrayValue(report['cases'])
		.map(objectValue)
		.filter((entry) => entry['id'] === caseId);
	assert.equal(selected.length, 1, 'Case input does not have exactly one declared case ID');
	const entry = selected[0];
	assert.ok(entry);
	const revision = textValue(objectValue(entry['case'])['revision']);
	const choices = candidatesFor(report, numberValue(entry['result_id'])).filter(
		(candidate) => candidate.revision === revision,
	);
	assert.equal(choices.length, 1, 'Case input does not link one revision/result');
	const candidate = choices[0];
	assert.ok(candidate);
	return candidate;
}
/** Change only case selectors on a private report copy; saved result payloads stay intact. */
export function renameSavedCase(report: JsonObject, oldId: string, newId: string): JsonObject {
	const renamed = structuredClone(report);
	const matches = arrayValue(renamed['cases'])
		.map(objectValue)
		.filter((entry) => entry['id'] === oldId);
	assert.equal(matches.length, 1, 'Numeric selector fixture requires one original case');
	const entry = matches[0];
	assert.ok(entry);
	entry['id'] = newId;
	objectValue(entry['case'])['id'] = newId;
	return renamed;
}
/** Remove only the colliding numeric rows in other revisions for the unique-owner control. */
export function retainNumericResultRevision(
	report: JsonObject,
	id: number,
	revision: string,
): JsonObject {
	const retained = structuredClone(report);
	for (const run of arrayValue(retained['runs']).map(objectValue)) {
		if (run['revision'] === revision) continue;
		const saved = objectValue(run['report']);
		for (const bucket of ['results', 'filter_answers', 'filter_skips'])
			saved[bucket] = arrayValue(saved[bucket]).filter((row) => objectValue(row)['id'] !== id);
	}
	assert.equal(candidatesFor(retained, id).length, 1, 'Unique numeric fixture is inert');
	return retained;
}
export function freezeCases(options: { source: string; sha256: string; evidence: string }): {
	file: string;
	report: JsonObject;
} {
	const original = filesystem.readFileSync(options.source);
	assert.equal(
		createHash('sha256').update(original).digest('hex'),
		options.sha256,
		'Declared cases input changed',
	);
	const file = path.join(options.evidence, 'cases-input.json');
	filesystem.writeFileSync(file, original);
	assert.deepEqual(filesystem.readFileSync(file), original);
	assert.deepEqual(
		filesystem.readFileSync(options.source),
		original,
		'Original cases input changed during copy',
	);
	const report = parseObject(original.toString('utf8'));
	writeJson(path.join(options.evidence, 'input-provenance.json'), {
		source: path.resolve(options.source),
		copy: file,
		sha256: options.sha256,
		bytes: original.length,
		cases: arrayValue(report['cases']).length,
		revisions: arrayValue(report['runs']).map((run) => objectValue(run)['revision']),
	});
	return { file, report };
}
