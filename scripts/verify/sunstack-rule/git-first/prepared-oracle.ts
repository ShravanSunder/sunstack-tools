/** Same-input full-capture oracle: external config and exact selected owners/requests. */
import assert from 'node:assert/strict';
import path from 'node:path';

import {
	arrayValue,
	objectValue,
	numberValue,
	textValue,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';

export function fullOracleConfig(
	config: JsonObject,
	configFile: string,
	maxFiles: number,
): JsonObject {
	assert.ok(
		Number.isInteger(maxFiles) && maxFiles > 2000,
		'Declare the raised full-capture limit before observing',
	);
	const result = structuredClone(config);
	const directory = path.dirname(configFile);
	for (const pair of arrayValue(result['rulebook']).map(objectValue)) {
		pair['markdown'] = path.resolve(directory, textValue(pair['markdown']));
		pair['rules'] = path.resolve(directory, textValue(pair['rules']));
	}
	const evidence = objectValue(result['evidence']);
	evidence['ast_grep'] = path.resolve(directory, textValue(evidence['ast_grep']));
	result['limits'] = { ...objectValue(result['limits'] ?? {}), max_files: maxFiles };
	return result;
}
/** Picked pieces are the selection authority, including files without a result/request. */
export function preparedSelectedFiles(report: JsonObject): string[] {
	const selected = new Set<string>();
	for (const row of arrayValue(report['file_counts']).map(objectValue)) {
		const kinds = Object.values(objectValue(row['kinds'])).map(objectValue);
		for (const kind of kinds) {
			const picked = numberValue(kind['picked']);
			assert.ok(Number.isInteger(picked) && picked >= 0, 'Invalid picked-piece count');
			if (picked > 0) selected.add(textValue(row['file']));
		}
	}
	return [...selected].toSorted();
}
/** File owners represented by saved rows are a separate account from picked files. */
export function preparedOwnerFiles(report: JsonObject): string[] {
	const owners = new Set<string>();
	for (const row of arrayValue(report['results']).map(objectValue))
		owners.add(textValue(row['file']));
	for (const row of arrayValue(report['requests']).map(objectValue)) {
		const owner = objectValue(row['owner']);
		switch (textValue(owner['kind'])) {
			case 'rule':
				owners.add(textValue(objectValue(owner['piece'])['file']));
				break;
			case 'selection':
				owners.add(textValue(owner['file']));
				break;
			case 'filter':
				break; // Folder ownership is not a checked-file identity.
			default:
				throw new Error('Unknown saved request owner kind');
		}
	}
	for (const row of arrayValue(report['selections']).map(objectValue))
		owners.add(textValue(objectValue(row['where'])['file']));
	return [...owners].toSorted();
}
export function requireSamePreparedInputs(options: {
	candidate: JsonObject;
	full: JsonObject;
	candidateVersion: string;
	fullVersion: string;
}): void {
	for (const [report, version] of [
		[options.candidate, options.candidateVersion],
		[options.full, options.fullVersion],
	] as const) {
		assert.equal(report['version'], version);
		assert.equal(report['mode'], 'prepare');
		assert.equal(
			objectValue(report['status'])['kind'],
			'prepared',
			'Oracle did not complete preparation',
		);
		assert.equal(
			objectValue(report['usage'])['model_requests'],
			0,
			'Keyless preparation dispatched',
		);
	}
	assert.ok(
		preparedSelectedFiles(options.full).length > 0,
		'Original-input oracle selected no files',
	);
	assert.deepEqual(
		preparedSelectedFiles(options.candidate),
		preparedSelectedFiles(options.full),
		'Selected file owners differ from full capture',
	);
	assert.deepEqual(
		preparedOwnerFiles(options.candidate),
		preparedOwnerFiles(options.full),
		'Represented file owners differ from full capture',
	);
	for (const field of [
		'results',
		'requests',
		'filter_answers',
		'filter_skips',
		'selections',
		'needs_answer',
	])
		assert.deepEqual(
			options.candidate[field],
			options.full[field],
			`Prepared ${field} differs from same-input full capture`,
		);
}
