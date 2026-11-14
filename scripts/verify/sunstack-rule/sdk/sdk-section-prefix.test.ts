import { describe, expect, test } from 'vitest';

import { requireSdkSectionPrefixPieces } from './sdk-section-prefix.ts';

function correctPieces(
	options: {
		firstValue?: string;
		secondValue?: string;
		secondEnd?: bigint;
		outsideReason?: string;
	} = {},
): Record<string, unknown>[] {
	const identity = 'f33c479351e145386757f84d49b6ba7c3d24bb3f7378265a1e55fcc5380e70cf';
	const prefix = 'function run() {\n  const value = 1;\n  ';
	return [
		{
			kind: 'subject',
			text: 'console.log(value)',
			fields: {
				preceding: {
					state: 'text',
					value: options.firstValue ?? prefix,
					spans: [
						{
							byte_start: 9n,
							byte_end: 47n,
							start_line: 2n,
							start_column: 1n,
							end_line: 4n,
							end_column: 3n,
							source: { content_identity: identity },
						},
					],
				},
				empty_prefix: { state: 'text', value: '', spans: [{ byte_start: 47n, byte_end: 47n }] },
			},
		},
		{
			kind: 'subject',
			text: 'console.log(other)',
			fields: {
				preceding: {
					state: 'text',
					value: options.secondValue ?? 'function nested() {\n    ',
					spans: [
						{
							byte_start: 69n,
							byte_end: options.secondEnd ?? 93n,
							start_line: 5n,
							start_column: 3n,
							end_line: 6n,
							end_column: 5n,
							source: { content_identity: identity },
						},
					],
				},
				empty_prefix: { state: 'text', value: '', spans: [{ byte_start: 93n, byte_end: 93n }] },
			},
		},
		{
			kind: 'subject',
			text: 'console.log(outside)',
			fields: {
				preceding: { state: 'unavailable', reason: options.outsideReason ?? 'unreadable' },
			},
		},
	];
}

describe('section prefix observation oracle', () => {
	test('accepts independently specified exact source facts', () => {
		expect(() => requireSdkSectionPrefixPieces(correctPieces())).not.toThrow();
	});
	test('rejects a full-section fallback, wrong bytes and a fabricated missing prefix', () => {
		expect(() =>
			requireSdkSectionPrefixPieces(
				correctPieces({ firstValue: 'function run() { full section }' }),
			),
		).toThrow('Wrong exact prefix');
		expect(() =>
			requireSdkSectionPrefixPieces(
				correctPieces({
					secondValue:
						'function run() {\n  const value = 1;\n  console.log(value);\n  function nested() {\n    ',
				}),
			),
		).toThrow('Wrong exact prefix');
		expect(() => requireSdkSectionPrefixPieces(correctPieces({ secondEnd: 92n }))).toThrow(
			'Wrong UTF-8 prefix span',
		);
		expect(() =>
			requireSdkSectionPrefixPieces(correctPieces({ outsideReason: 'selection_pending' })),
		).toThrow('Missing container');
	});
});
