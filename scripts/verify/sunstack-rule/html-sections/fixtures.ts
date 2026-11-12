/** Owned outline and asking controls, distinct from the three frozen real pages. */
import filesystem from 'node:fs';
import path from 'node:path';

import { writeProject } from '../html-evidence/html-fixtures.ts';
import { type JsonObject } from '../rulebook/rulebook-fixture-support.ts';
import type { Section } from './corpus.ts';

export const sectionFields = {
	full: { from: 'section', text: 'full' },
	own: { from: 'section', text: 'own' },
	parent: { from: 'section', text: 'enclosing_own' },
};
export const askingRule: JsonObject = {
	name: 'section-picture',
	evidence: 'section',
	severity: 'warn',
	questions: {
		quality: {
			type: 'noul',
			instructions: {
				check: 'full',
				context: ['own', 'parent'],
				task: 'Does the explanation have its picture?',
			},
		},
	},
	decision: {
		start: 'quality',
		steps: {
			quality: {
				ask: 'quality',
				yes_at_or_above: 0.8,
				no_at_or_below: 0.2,
				yes: 'pass',
				['no']: { finding: 'Inspect the picture.' },
				uncertain: 'uncertain',
			},
		},
	},
};
export function prepareProject(options: {
	root: string;
	files: Readonly<Record<string, string>>;
	maxChars?: number;
}): void {
	filesystem.mkdirSync(options.root);
	for (const [file, text] of Object.entries(options.files))
		filesystem.writeFileSync(path.join(options.root, file), text);
	writeProject(
		options.root,
		{ K001: askingRule },
		{
			section: {
				html_units: { mode: 'unit', units: ['section'] },
				fields: sectionFields,
				max_chars: options.maxChars ?? 300_000,
			},
		},
		Object.keys(options.files),
		'id: whole\nlanguage: Html\nrule: {kind: document}\n',
	);
}
export const edgeHtml = [
	'<h1 id="root">Root 文</h1><p>Root introduction.</p><img alt="Root image"><span aria-label="Root accessible">Root picture.</span>',
	'<!-- <h2>Comment fake</h2> --><template><h2>Template fake</h2></template>',
	'<script>const example = "<h2>Script fake</h2>";</script>',
	'<ex-markdown><script type="text/markdown"># Island heading\n<h2>Island fake</h2></script></ex-markdown>',
	'<div><h2 id="parent">Parent</h2>',
	...Array.from({ length: 7 }, (_, index) => `<p>Block ${index + 1}: ${'prose '.repeat(35)}</p>`),
	'<ex-mermaid label="Late diagram"><script type="text/plain">graph TD; A-->B;</script></ex-mermaid></div>',
	'<h4 id="child">Duplicate</h4><p>Child own.</p>',
	'<section><h6 id="deep">Deep</h6><p>Deep own.</p></section>',
	'<h3 id="sibling">Sibling</h3><p>Sibling own.</p>',
	'<h5 id="other">Duplicate</h5><p>Other own.</p>',
	'<h1 id="last">Last</h1><p>EOF tail.</p>',
].join('\n');
export function edgeSections(): Section[] {
	const markers = [
		{ id: 'root', level: 1, parent: null, end: 6 },
		{ id: 'parent', level: 2, parent: 0, end: 6 },
		{ id: 'child', level: 4, parent: 1, end: 4 },
		{ id: 'deep', level: 6, parent: 2, end: 4 },
		{ id: 'sibling', level: 3, parent: 1, end: 6 },
		{ id: 'other', level: 5, parent: 4, end: 6 },
		{ id: 'last', level: 1, parent: null, end: 7 },
	];
	const starts = markers.map((marker) =>
		Buffer.byteLength(edgeHtml.slice(0, edgeHtml.indexOf(`<h${marker.level} id="${marker.id}">`))),
	);
	return markers.map((marker, index) => ({
		level: marker.level,
		start: starts[index] ?? -1,
		end: starts[marker.end] ?? Buffer.byteLength(edgeHtml),
		own_end: starts[index + 1] ?? Buffer.byteLength(edgeHtml),
		parent: marker.parent,
		heading: marker.id,
	}));
}
export const projectionHtml =
	'<h2>A</h2><p>B</p><img alt="Image &amp; meaning"><ex-mermaid label="Flow"><script type="text/plain">A-->B</script></ex-mermaid><span aria-label="Accessible">C</span>';
export const projectionText =
	'A B [alt: Image & meaning] [label: Flow] A-->B [aria-label: Accessible] C';

// Each separate selection/shared/filter view fits the admitted1000-character limit only after projection.
export const askingPathHtml = projectionHtml.repeat(8);
export const askingPathText = Array.from({ length: 8 }, () => projectionText).join(' ');
