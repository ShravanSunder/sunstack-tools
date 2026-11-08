import { expect, test } from 'vitest';

import {
	requireComparableBytes,
	requireHumanParity,
	requireHumanSummary,
} from './rulebook-output-comparison.ts';

const entries = [
	{ id: 7, file: 'a.md', line: 1, kind: 'unsure', rule: 'first.rule' },
	{ id: 8, file: 'b.md', line: 2, kind: 'unsure', rule: 'first.rule' },
	{ id: 9, file: 'c.md', line: 3, kind: 'unsure', rule: 'second' },
] as const;
const summary =
	'2 unsure [first.rule]; details: lint show <report> 8\n1 unsure [second]; details: lint show <report> 9\n';
test('unsure groups retain exact counts and any valid same-rule pointer', () => {
	expect(() => requireHumanSummary(summary, entries)).not.toThrow();
});
test.each([
	['wrong count', summary.replace('2 unsure', '1 unsure')],
	['another rule pointer', summary.replace('<report> 8', '<report> 9')],
	['missing rule', summary.split('\n')[0] ?? ''],
	['duplicated rule', `${summary}${summary}`],
	['old individual rows', 'a.md:1: unsure [first.rule] at step quality #7\n'],
	[
		'individual rows beside summaries',
		`${summary}a.md:1: unsure [first.rule] at step quality #7\n`,
	],
])('unsure summaries reject %s', (_name, text) => {
	expect(() => requireHumanSummary(text, entries)).toThrow();
});
const before = Buffer.from('{\n  "version": "0.9.0",\n  "count": 68\n}');
const after = Buffer.from('{\n  "version": "0.10.0",\n  "count": 68\n}');
test('historical comparison permits exactly the identified root version field', () => {
	expect(() =>
		requireComparableBytes({
			before,
			after,
			format: 'json',
			baselineVersion: '0.9.0',
			candidateVersion: '0.10.0',
		}),
	).not.toThrow();
});
test.each([
	['data change', Buffer.from(after.toString().replace('68', '67')), '0.9.0', 'json'],
	['whitespace change', Buffer.from(`${after.toString()}\n`), '0.9.0', 'json'],
	['same-version normalization', after, '0.10.0', 'json'],
	['versionless format', after, '0.9.0', 'show'],
] as const)('byte comparison rejects %s', (_name, changed, baselineVersion, format) => {
	expect(() =>
		requireComparableBytes({
			before,
			after: changed,
			format,
			baselineVersion,
			candidateVersion: '0.10.0',
		}),
	).toThrow();
});
test('human comparison preserves every line outside unsure output', () => {
	const oldText = Buffer.from(
		'a.md:1: warn [rule] Inspect. #3\na.md:1: unsure [rule] at step q #7\nUncertain: 0 error(s), 1 warning(s)\n',
	);
	const newText = Buffer.from(
		'a.md:1: warn [rule] Inspect. #3\n1 unsure [rule]; details: lint show <report> 7\nUncertain: 0 error(s), 1 warning(s)\n',
	);
	expect(() => requireHumanParity(oldText, newText)).not.toThrow();
	expect(() =>
		requireHumanParity(oldText, Buffer.from(newText.toString().replace('Inspect.', 'Changed.'))),
	).toThrow();
});
