import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { type DefaultTreeAdapterMap, parseFragment } from 'parse5';
import { expect, it } from 'vitest';

import { normalizeIslandText } from './source-text.ts';

it('preserves Markdown and Mermaid structure after HTML formatter indentation', (): void => {
	expect(normalizeIslandText('\n\t\t# Design\n\n\t\tA paragraph.\n\t')).toBe(
		'# Design\n\nA paragraph.',
	);
	expect(normalizeIslandText('\n\tflowchart TD\n\t  A --> B\n')).toBe('flowchart TD\n  A --> B');
});

function formatHtmlSource(source: string): string {
	const result = spawnSync(
		fileURLToPath(new URL('../../../../node_modules/.bin/oxfmt', import.meta.url)),
		[
			'--stdin-filepath',
			fileURLToPath(new URL('../../../../island-format-example.html', import.meta.url)),
			'--config',
			fileURLToPath(new URL('../../../../.oxfmtrc.json', import.meta.url)),
		],
		{ input: source, encoding: 'utf8', timeout: 5000 },
	);
	expect(result.status, result.stderr).toBe(0);
	return result.stdout;
}

function textOf(parent: DefaultTreeAdapterMap['parentNode']): string {
	return parent.childNodes
		.map((node): string => {
			if ('value' in node) return node.value;
			return 'childNodes' in node ? textOf(node) : '';
		})
		.join('');
}

/** Read the whole carrier, including whitespace; literal text gets no trimming/dedenting. */
function carrierText(html: string): string {
	const owner = parseFragment(html).childNodes.find(
		(node): node is DefaultTreeAdapterMap['element'] | DefaultTreeAdapterMap['template'] =>
			'tagName' in node && node.tagName.startsWith('ex-'),
	);
	const wrapper = owner?.childNodes.find(
		(node): node is DefaultTreeAdapterMap['element'] | DefaultTreeAdapterMap['template'] =>
			'tagName' in node && ['script', 'template'].includes(node.tagName),
	);
	if (!wrapper) throw new Error('Expected an island source carrier.');
	return textOf('content' in wrapper ? wrapper.content : wrapper);
}

it('retains hierarchical Markdown through the actual configured HTML formatter', (): void => {
	const source = '# Design\n\n- Parent\n  - Child\n';
	const formatted = formatHtmlSource(
		`<ex-tree><script type="text/markdown">\n${source}</script></ex-tree>`,
	);
	expect(normalizeIslandText(carrierText(formatted))).toBe(source.trim());
});

it('keeps the documented literal example exact through the actual formatter, without normalization', (): void => {
	const guide = readFileSync(new URL('../../skill/references/trees.md', import.meta.url), 'utf8');
	const example = [...guide.matchAll(/```html\s*([\s\S]*?)```/g)]
		.map((match): string => match[1] ?? '')
		.find((snippet): boolean => snippet.includes('label="Literal markup"'));
	if (!example) throw new Error('The guide must retain its literal example.');
	expect(carrierText(formatHtmlSource(example))).toBe(
		'first line\n</script>\n<widget>& value\nlast line',
	);
});
