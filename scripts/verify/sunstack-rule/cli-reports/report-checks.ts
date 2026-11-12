/** Preserved saved payloads and structured refusal paths; no production decoder is used. */
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import {
	arrayValue,
	numberValue,
	objectValue,
	textValue,
	type JsonObject,
	type JsonValue,
} from '../rulebook/rulebook-fixture-support.ts';
import type { SavedCandidate } from './report-inputs.ts';

export function requireLintDetail(actual: JsonObject, candidate: SavedCandidate): void {
	assert.deepEqual(
		Object.keys(actual).toSorted(),
		['result', 'finding', 'requests', 'evidence', 'rulebook_files', 'rewalkable', 'gap'].toSorted(),
		'Unexpected public detail shape; return it to Lead before adapting',
	);
	assert.deepEqual(actual['result'], candidate.result, 'Saved result owner/content changed');
	const ids = arrayValue(candidate.result['request_ids']);
	const requests = arrayValue(candidate.report['requests'])
		.map(objectValue)
		.filter((request) => ids.includes(request['id'] ?? null));
	assert.deepEqual(
		actual['requests'],
		requests,
		'Saved questions/state/answers changed or unrelated requests leaked',
	);
	assert.deepEqual(actual['rulebook_files'], candidate.report['rulebook_files']);
	const evidence =
		objectValue(candidate.report['piece_evidence'])[String(numberValue(candidate.result['id']))] ??
		null;
	assert.deepEqual(actual['evidence'], evidence, 'Saved raw evidence changed');
	const finding =
		arrayValue(candidate.report['findings'])
			.map(objectValue)
			.find((item) => item['id'] === candidate.result['id']) ?? null;
	assert.deepEqual(actual['finding'], finding);
	assert.equal(actual['rewalkable'], true, 'Declared positive result lost its rewalkable detail');
	assert.equal(actual['gap'], null);
}
export function requireCaseDetail(
	actual: JsonObject,
	candidate: SavedCandidate,
	expectedCase: JsonObject,
): void {
	assert.deepEqual(
		Object.keys(actual).toSorted(),
		['case', 'revision', 'detail'].toSorted(),
		'Unexpected public case envelope; return to Lead',
	);
	assert.deepEqual(actual['case'], expectedCase);
	assert.equal(actual['revision'], candidate.revision);
	requireLintDetail(objectValue(actual['detail']), candidate);
}
export function requireAmbiguity(
	output: string,
	id: number,
	candidates: readonly SavedCandidate[],
): void {
	assert.match(output, /case/i);
	assert.match(output, /ambig|multiple|more than one/i);
	assert.ok(new RegExp(`\\b${id}\\b`).test(output), 'Ambiguity did not name the requested ID');
	assert.ok(candidates.length > 1, 'Ambiguity control is inert');
	for (const candidate of candidates) {
		assert.ok(output.includes(candidate.revision), 'Candidate revision omitted');
		const resultId = numberValue(candidate.result['id']);
		assert.ok(new RegExp(`\\b${resultId}\\b`).test(output), 'Candidate result ID omitted');
		if (candidate.caseIds.length === 0) assert.match(output, /no case id/i);
		for (const caseId of candidate.caseIds)
			assert.ok(output.includes(caseId), 'Candidate case ID omitted');
	}
	assert.doesNotMatch(
		output,
		/missing field|invalid type:|unknown variant| at line \d+ column \d+/i,
		'Raw serde error leaked',
	);
}
function samePath(value: JsonValue | undefined, root: string, expected: string): boolean {
	if (typeof value !== 'string') return false;
	try {
		const local = value.startsWith('file:') ? fileURLToPath(value) : decodeURIComponent(value);
		return path.resolve(root, local) === path.resolve(root, expected);
	} catch {
		return false;
	}
}
export interface ExpectedRefusal {
	readonly path: string;
	readonly reason: RegExp;
	readonly limit?: { readonly name: string; readonly value: number };
}
function requireMessage(message: string, expected: ExpectedRefusal): void {
	assert.match(message, expected.reason, 'Wrong change-scope refusal reason');
	if (expected.limit) {
		const words = expected.limit.name
			.split(/[_\s-]+/)
			.map((word) => word.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
		const namedValue = new RegExp(
			`\\b(?:max[_\\s-]*)?${words.join('[_\\s-]*')}\\b[^\\d\\n]{0,64}${expected.limit.value}(?!\\d)`,
			'i',
		);
		assert.match(
			message,
			namedValue,
			'Limit stop omitted the configured value next to its limit name',
		);
	}
}
export function requireJsonRefusal(
	report: JsonObject,
	root: string,
	expected: ExpectedRefusal,
): void {
	const diagnostics = arrayValue(report['diagnostics']).map(objectValue);
	const selected = diagnostics.filter(
		(entry) =>
			samePath(entry['path'], root, expected.path) || samePath(entry['file'], root, expected.path),
	);
	assert.ok(
		selected.length > 0,
		'JSON has no offending path field; message text alone is insufficient',
	);
	for (const entry of selected) requireMessage(textValue(entry['message']), expected);
}
export function requireSarifRefusal(
	report: JsonObject,
	root: string,
	expected: ExpectedRefusal,
): void {
	const entries = arrayValue(report['runs'])
		.map(objectValue)
		.flatMap((run) => {
			const results = arrayValue(run['results']).map(objectValue);
			const notifications = arrayValue(run['invocations'])
				.map(objectValue)
				.flatMap((invocation) =>
					arrayValue(invocation['toolExecutionNotifications']).map(objectValue),
				);
			return results.concat(notifications);
		});
	const selected = entries.filter((entry) => {
		const properties = entry['properties'] ? objectValue(entry['properties']) : {};
		if (samePath(properties['path'], root, expected.path)) return true;
		const locations = arrayValue(entry['locations'] ?? []).map(objectValue);
		return locations.some((location) => {
			const physical = objectValue(location['physicalLocation']);
			return samePath(objectValue(physical['artifactLocation'])['uri'], root, expected.path);
		});
	});
	assert.ok(selected.length > 0, 'SARIF has no offending artifact URI/path field');
	for (const entry of selected)
		requireMessage(textValue(objectValue(entry['message'])['text']), expected);
}
export function requireHumanRefusal(output: string, expected: ExpectedRefusal): void {
	assert.ok(
		output.includes(expected.path),
		'Human refusal does not name the offending relative path',
	);
	requireMessage(output, expected);
}
