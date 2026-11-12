/** Status-specific CLI assertions; JSON parity stays in the existing byte comparator. */
import assert from 'node:assert/strict';

import {
	arrayValue,
	objectValue,
	numberValue,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';

export function requireScope(text: string, all: boolean): void {
	const lines = text.split(/\r?\n/).filter((line) => line.startsWith('Scope:'));
	assert.equal(lines.length, 1, 'Scope line missing or repeated');
	if (all) assert.equal(lines[0], 'Scope: all configured targets');
}
export function requireProblemSummary(text: string, status: string, count: number): void {
	const lines = text.split(/\r?\n/).filter((line) => line.startsWith(status + ':'));
	assert.equal(lines.length, 1, 'Status summary missing or repeated');
	const summary = lines[0];
	assert.ok(summary);
	assert.doesNotMatch(
		summary,
		/error\(s\)|warning\(s\)/,
		'Finding counts leaked into a status without finding counts',
	);
	assert.match(
		summary,
		new RegExp(`\\b${count} (?:problem|diagnostic)s?\\b`),
		'Diagnostic count omitted from status summary',
	);
}
export function requirePreparedSummary(text: string, count: number, file: string): void {
	const lines = text.split(/\r?\n/).filter((line) => line.startsWith('Prepared '));
	assert.equal(lines.length, 1, 'Prepared request summary missing');
	const summary = lines[0];
	assert.ok(summary);
	assert.match(summary, new RegExp(`^Prepared ${count} request${count === 1 ? '' : 's'}\\b`));
	assert.ok(
		summary.includes(' in ' + file + ':'),
		'Prepared summary omitted its actual output path',
	);
	assert.match(summary, /error\(s\).*warning\(s\)/, 'Prepared finding counts changed');
}
export function requireCredentialRefusal(report: JsonObject): void {
	assert.equal(objectValue(report['status'])['kind'], 'invalid');
	assert.ok(
		arrayValue(report['diagnostics'])
			.map(objectValue)
			.some((item) => item['code'] === 'credential_missing'),
	);
	assert.equal(numberValue(objectValue(report['usage'])['model_requests']), 0);
	for (const field of ['results', 'requests', 'findings', 'file_counts', 'selections'])
		assert.equal(arrayValue(report[field]).length, 0, `Credential refusal left partial ${field}`);
}
export function requireNoSelection(report: JsonObject): void {
	assert.equal(objectValue(report['status'])['kind'], 'nothing_selected');
	assert.equal(numberValue(objectValue(report['usage'])['model_requests']), 0);
	for (const field of ['results', 'requests', 'file_counts', 'selections'])
		assert.equal(arrayValue(report[field]).length, 0);
}
export function requireConfigReason(text: string, file: string, reason: RegExp): void {
	assert.ok(text.includes(file), 'Config refusal omitted the supplied path');
	assert.match(text, reason, 'Config refusal omitted its actual open failure reason');
	assert.doesNotMatch(
		text,
		/selected sunstack\.lint\.json/,
		'Config message substituted the default basename',
	);
}
export function requireHelp(text: string): void {
	const lines = text.split(/\r?\n/);
	const start = lines.findIndex((line) => /^\s+--config(?:\s|=)/.test(line));
	assert.ok(start >= 0);
	const next = lines.findIndex(
		(line, index) => index > start && /^\s+(?:--[a-z]|-[A-Za-z](?:,|\s))/.test(line),
	);
	const config = lines.slice(start, next < 0 ? undefined : next).join(' ');
	assert.match(config, /config(?:uration)? file/i);
	assert.match(config, /nearest/i);
	assert.match(config, /sunstack\.lint\.json/);
	assert.match(config, /upward|parent/i);
	const match = /exit codes?\s*:/i.exec(text);
	assert.ok(match, 'Exit-code footer missing');
	const footer = text.slice(match.index + match[0].length);
	for (const [code, meaning] of [
		[0, 'clean|nothing selected'],
		[1, 'finding'],
		[2, 'invalid'],
		[3, 'incomplete'],
		[4, 'cancel'],
		[5, 'uncertain'],
		[6, 'prepared'],
	] as const)
		assert.match(
			footer,
			new RegExp(`(?:^|\\n)\\s*${code}\\s+[^\\n]*(?:${meaning})`, 'i'),
			`Exit ${code} missing or mislabeled`,
		);
}
