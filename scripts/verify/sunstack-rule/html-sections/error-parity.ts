/** C3/C4 source-literal restorations. Nothing else in a prepared report may move. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import {
	arrayValue,
	numberValue,
	objectValue,
	textValue,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';

export function restoreArrowWords(text: string, isJ1Corpus: boolean): string {
	const replacements = [
		[
			'Your words: "workspace -',
			'Your words: "workspace -> Areas -> Projects"; "board is not a good name". An',
		],
		[
			'Your words: "common reference system sourceEntity uuidv7 -',
			'Your words: "common reference system sourceEntity uuidv7 -> destination uuidv7"; "don\'t prescribe too much structure… json". A backlog item, thread, post, log entry or deliverable can',
		],
		[
			'Your words: "deliverable -',
			`Your words: "deliverable -> implement verifier reviewer". ${isJ1Corpus ? 'Parallel deliverables therefore never share a set. ' : ''}Roles are plain text, and each has at most one holder at a time ("roles can have one of each as enforcement"; "its not an enum enforced for roles its just text").`,
		],
		[
			'"we can allow a thread to have links -',
			'"we can allow a thread to have links -> deliverable"; "creator -> resolves thread". Today, 61 of 64 resolves were done by the agent running the thread.',
		],
		[
			'"deliverable has blocked by -',
			'"deliverable has blocked by -> blocking so it can be sequenced". Later, not now: a deliverable may carry assets in R2, such as HTML pages ("we talk later").',
		],
	] as const;
	let restored = text;
	for (const [before, after] of replacements) restored = restored.replaceAll(before, after);
	return restored;
}

export function requireRecoveringParity(
	before: Buffer,
	after: Buffer,
	isJ1Corpus: boolean,
): JsonObject[] {
	const expected = objectValue(JSON.parse(before.toString('utf8')));
	const actual = objectValue(JSON.parse(after.toString('utf8')));
	assert.equal(expected['version'], '0.14.0');
	assert.equal(actual['version'], '0.14.1');
	const suffix = before.toString('utf8').endsWith('\n') ? '\n' : '';
	assert.equal(
		JSON.stringify(expected, null, 2) + suffix,
		before.toString('utf8'),
		'Serializer cannot preserve baseline bytes',
	);
	expected['version'] = '0.14.1';
	const differences: JsonObject[] = [];
	const pieces = objectValue(expected['piece_evidence']);
	for (const [id, value] of Object.entries(pieces)) {
		const piece = objectValue(objectValue(value)['piece']);
		const file = textValue(objectValue(piece['where'])['file']);
		if (file !== (isJ1Corpus ? 'pages/page-03.html' : 'wrapped.html')) continue;
		const view = objectValue(objectValue(piece['model_view'])['fields']);
		for (const [field, rawValue] of Object.entries(view)) {
			if (typeof rawValue !== 'string') continue;
			const restored = restoreArrowWords(rawValue, isJ1Corpus);
			if (restored !== rawValue) {
				view[field] = restored;
				differences.push({ id, file, field, before: rawValue, after: restored });
			}
		}
	}
	for (const value of arrayValue(expected['requests'])) {
		const record = objectValue(value);
		const piece = objectValue(objectValue(pieces[String(numberValue(record['id']))])['piece']);
		const file = textValue(objectValue(piece['where'])['file']);
		if (file !== (isJ1Corpus ? 'pages/page-03.html' : 'wrapped.html')) continue;
		const request = objectValue(record['request']);
		assert.equal(
			record['input_sha256'],
			createHash('sha256').update(JSON.stringify(request)).digest('hex'),
		);
		const state = objectValue(request['state']);
		let changed = false;
		for (const field of ['full', 'own', 'parent']) {
			const original = state[field];
			if (typeof original !== 'string') continue;
			const restored = restoreArrowWords(original, isJ1Corpus);
			state[field] = restored;
			changed ||= restored !== original;
		}
		if (changed)
			record['input_sha256'] = createHash('sha256').update(JSON.stringify(request)).digest('hex');
	}
	assert.ok(differences.length > 0, 'Real page must exercise recovery');
	assert.equal(
		JSON.stringify(expected, null, 2) + suffix,
		after.toString('utf8'),
		'Report changed beyond exact source-word restorations and their computed hashes',
	);
	return differences;
}

export function requireSourceWords(source: string, asking: string): void {
	const actual = asking.split(/\s+/u);
	let cursor = 0;
	for (const word of source.split(/\s+/u).filter(Boolean)) {
		while (cursor < actual.length && actual[cursor] !== word) cursor++;
		assert.ok(cursor < actual.length, `Source word absent or reordered: ${word}`);
		cursor++;
	}
}
