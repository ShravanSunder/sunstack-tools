/** Validate the immutable score manifest and repository-only execution copies. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import { compactJson, jsonSlice } from '../prepare/raw-json.ts';
import {
	arrayValue,
	objectValue,
	parseObject,
	textValue,
} from '../rulebook/rulebook-fixture-support.ts';
import { requireManifestSelection } from './case-dispatch-checks.ts';

export interface CorpusFileSelection {
	readonly file: string;
	readonly originalSha256: string;
	readonly executionSha256: string;
	readonly repository: string;
	readonly scoreIds: readonly string[];
	readonly heldIds: readonly string[];
	readonly pendingIds: readonly string[];
	readonly unmatchedIds: readonly string[];
	readonly notCollectibleIds: readonly string[];
}

/** The copy may rebase only repository; expected labels never become SDK bindings. */
export function readCorpusSelections(
	manifestPath: string,
	copiesDirectory: string,
): CorpusFileSelection[] {
	assert.ok(path.isAbsolute(manifestPath) && path.isAbsolute(copiesDirectory));
	const manifest = parseObject(filesystem.readFileSync(manifestPath, 'utf8'));
	const files = objectValue(manifest['files']);
	const results: CorpusFileSelection[] = [];
	for (const [file, rawCategories] of Object.entries(files)) {
		assert.equal(path.basename(file), file, 'Manifest file escapes its case directory');
		const originalRaw = filesystem.readFileSync(
			path.join(path.dirname(manifestPath), file),
			'utf8',
		);
		const copyRaw = filesystem.readFileSync(path.join(copiesDirectory, file), 'utf8');
		const original = parseObject(originalRaw);
		const copy = parseObject(copyRaw);
		const originalKeys = Object.keys(original)
			.filter((key) => key !== 'repository')
			.toSorted();
		const copyKeys = Object.keys(copy)
			.filter((key) => key !== 'repository')
			.toSorted();
		assert.deepEqual(originalKeys, copyKeys, 'Execution copy changed case-file fields');
		for (const key of originalKeys)
			assert.equal(
				compactJson(jsonSlice(originalRaw, [key])),
				compactJson(jsonSlice(copyRaw, [key])),
				`Execution copy changed ${key}`,
			);
		const repository = textValue(copy['repository']);
		assert.ok(
			path.isAbsolute(repository) && filesystem.statSync(repository).isDirectory(),
			'Copied repository must exist',
		);
		const categories = objectValue(rawCategories);
		const strings = (key: string): string[] => arrayValue(categories[key] ?? []).map(textValue);
		const records = (key: string): string[] =>
			arrayValue(categories[key] ?? []).map((value) => textValue(objectValue(value)['id']));
		const selection = {
			scoreIds: strings('score'),
			heldIds: strings('held'),
			pendingIds: strings('pending'),
			unmatchedIds: records('unmatched'),
			notCollectibleIds: records('not_collectible'),
		};
		const caseIds = arrayValue(original['cases']).map((value) =>
			textValue(objectValue(value)['id']),
		);
		requireManifestSelection({ ...selection, caseIds });
		results.push({
			file,
			originalSha256: createHash('sha256').update(originalRaw).digest('hex'),
			executionSha256: createHash('sha256').update(copyRaw).digest('hex'),
			repository,
			...selection,
		});
	}
	return results;
}

/** These flags intersect the manifest score list with development, never --set alone. */
export function scoreSelectionArguments(selection: CorpusFileSelection): string[] {
	assert.ok(selection.scoreIds.length > 0, 'No score IDs supplied');
	return ['--set', 'development', ...selection.scoreIds.flatMap((id) => ['--case', id])];
}
