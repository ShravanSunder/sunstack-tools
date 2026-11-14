import { describe, expect, test } from 'vitest';

import { requireSdkIslandPieces } from './sdk-embedded-markdown.ts';

function islandPieces(): Record<string, unknown>[] {
	const source = {
		path: 'docs/island.html',
		content_identity: 'c8da843a60cb29451671312e117f63ba5f918eec830e6915c173da4f61ef7643',
	};
	return [
		{
			kind: 'html',
			text: '<p>Before 文.</p>',
			where: {
				file: 'docs/island.html',
				span: {
					source,
					byte_start: 0n,
					byte_end: 18n,
					start_line: 1n,
					start_column: 1n,
					end_line: 1n,
					end_column: 17n,
				},
			},
		},
		{
			kind: 'markdown_ast',
			text: 'Paragraph café.\r\n',
			where: {
				file: 'docs/island.html',
				span: {
					source,
					byte_start: 55n,
					byte_end: 73n,
					start_line: 3n,
					start_column: 1n,
					end_line: 4n,
					end_column: 1n,
				},
			},
		},
		{
			kind: 'markdown_unit',
			text: 'Paragraph café.\r\n',
			where: {
				file: 'docs/island.html',
				span: {
					source,
					byte_start: 55n,
					byte_end: 73n,
					start_line: 3n,
					start_column: 1n,
					end_line: 4n,
					end_column: 1n,
				},
			},
		},
		{
			kind: 'html',
			text: '<p>After.</p>',
			where: {
				file: 'docs/island.html',
				span: {
					source,
					byte_start: 130n,
					byte_end: 143n,
					start_line: 6n,
					start_column: 1n,
					end_line: 6n,
					end_column: 14n,
				},
			},
		},
	];
}

describe('configured Markdown island observation oracle', () => {
	test('accepts configured source order and omitted-settings controls', () => {
		const configured = islandPieces();
		expect(() => requireSdkIslandPieces(configured, true)).not.toThrow();
		expect(() => requireSdkIslandPieces([configured[0], configured[3]], false)).not.toThrow();
	});
	test('rejects lost islands, wrong order and a byte-based Unicode column', () => {
		const configured = islandPieces();
		expect(() => requireSdkIslandPieces([configured[0], configured[3]], true)).toThrow('count');
		expect(() =>
			requireSdkIslandPieces([configured[0], configured[2], configured[1], configured[3]], true),
		).toThrow('order/text');
		const wrongColumn = islandPieces();
		wrongColumn[0] = {
			kind: 'html',
			text: '<p>Before 文.</p>',
			where: {
				file: 'docs/island.html',
				span: {
					source: {
						path: 'docs/island.html',
						content_identity: 'c8da843a60cb29451671312e117f63ba5f918eec830e6915c173da4f61ef7643',
					},
					byte_start: 0n,
					byte_end: 18n,
					start_line: 1n,
					start_column: 1n,
					end_line: 1n,
					end_column: 19n,
				},
			},
		};
		expect(() => requireSdkIslandPieces(wrongColumn, true)).toThrow('coordinates');
	});
	test('requires SDK coordinates to retain their bigint type', () => {
		const pieces = islandPieces();
		pieces[0] = {
			kind: 'html',
			text: '<p>Before 文.</p>',
			where: {
				file: 'docs/island.html',
				span: {
					source: {
						path: 'docs/island.html',
						content_identity: 'c8da843a60cb29451671312e117f63ba5f918eec830e6915c173da4f61ef7643',
					},
					byte_start: 0n,
					byte_end: 18,
					start_line: 1n,
					start_column: 1n,
					end_line: 1n,
					end_column: 17n,
				},
			},
		};
		expect(() => requireSdkIslandPieces(pieces, true)).toThrow('wrong numeric type');
	});
});
