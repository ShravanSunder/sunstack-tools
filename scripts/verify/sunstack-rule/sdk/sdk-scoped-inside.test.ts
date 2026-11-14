import { describe, expect, test } from 'vitest';

import { requireScopedInsidePieces } from './sdk-scoped-inside.ts';

const identity = 'e93e99b0b0646be607bc6407a910f58ad563c639bd64d01257c547879a823f2b';
const before = "<img src='before'>";
const inside = "<img src='inner'>";
const after = "<img src='after'>";
const fence = '```mermaid\ngraph LR\n A --> B\n```\n';
const own = "<ex-choice id='own'><img src='inner'>Choose</ex-choice>";
const empty = "<ex-choice id='empty'>Choose</ex-choice>";

function span(
	byteStart: bigint,
	byteEnd: bigint,
	startLine: bigint,
	startColumn: bigint,
	endLine: bigint,
	endColumn: bigint,
): object {
	return {
		source: { path: 'docs/scoped.html', content_identity: identity },
		byte_start: byteStart,
		byte_end: byteEnd,
		start_line: startLine,
		start_column: startColumn,
		end_line: endLine,
		end_column: endColumn,
	};
}
const beforeSpan = span(47n, 65n, 2n, 13n, 2n, 31n);
const fenceSpan = span(96n, 129n, 4n, 1n, 8n, 1n);
const insideSpan = span(159n, 176n, 9n, 21n, 9n, 38n);
const afterSpan = span(194n, 211n, 9n, 56n, 9n, 73n);
function texts(values: readonly string[], spans: readonly object[]): object {
	return { state: 'texts', values, spans };
}
function fixture(): Record<string, unknown>[] {
	return [
		{
			kind: 'choice',
			text: own,
			fields: {
				html_before: texts([before], [beforeSpan]),
				markdown_before: texts([fence], [fenceSpan]),
				html_section: texts([before, inside, after], [beforeSpan, insideSpan, afterSpan]),
				markdown_section: texts([fence], [fenceSpan]),
				body: { state: 'text', value: own },
			},
			model_view: { state: 'bounded', fields: { markdown_before: [fence] }, cut: [] },
		},
		{
			kind: 'choice',
			text: empty,
			fields: {
				html_before: texts([], []),
				markdown_before: texts([], []),
				html_section: texts([], []),
				markdown_section: texts([], []),
			},
		},
		{
			kind: 'legacy',
			text: own,
			fields: { by_kind: texts([inside], [insideSpan]), by_rule: texts([inside], [insideSpan]) },
		},
		{ kind: 'legacy', text: empty, fields: { by_kind: texts([], []), by_rule: texts([], []) } },
	];
}

describe('scoped containment observation oracle', () => {
	test('accepts exact cross-language, empty and legacy results', () => {
		expect(() => requireScopedInsidePieces(fixture())).not.toThrow();
	});
	test('rejects whole-document leakage, lost scope and a wrong legacy result', () => {
		const leaked = fixture();
		leaked[0] = { ...leaked[0], fields: { html_before: texts(["<img src='old'>", before], []) } };
		expect(() => requireScopedInsidePieces(leaked)).toThrow('html_before values');
		const lostFence = fixture();
		lostFence[0] = {
			...lostFence[0],
			fields: { html_before: texts([before], [beforeSpan]), markdown_before: texts([], []) },
		};
		expect(() => requireScopedInsidePieces(lostFence)).toThrow('markdown_before values');
		const wrongLegacy = fixture();
		wrongLegacy[2] = {
			...wrongLegacy[2],
			fields: { by_kind: texts([], []), by_rule: texts([inside], [insideSpan]) },
		};
		expect(() => requireScopedInsidePieces(wrongLegacy)).toThrow('by_kind values');
	});
});
