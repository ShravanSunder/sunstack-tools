import { readFile } from 'node:fs/promises';

import { renderMermaidSVG } from 'beautiful-mermaid';

import { decodeEscapedHtmlText } from '../core/html-text.ts';
import {
	markdownSourceMarkup,
	parseMarkdownSource,
	type MarkdownSource,
} from '../core/markdown-source.ts';
import { normalizeIslandText } from '../core/source-text.ts';
import { subjectIcons } from '../core/subject-icons.ts';
import { renderThemedDiagram } from '../render/diagram.ts';
import { renderMarkdownContent } from '../render/markdown-content.ts';
import { embedMarkdownFiles } from './markdown-files.ts';

function readMarkdownRecord(sourceHtml: string): MarkdownSource {
	const retained =
		/<script type="application\/json" data-ex-markdown-source(?:="")?>([\s\S]*?)<\/script>/.exec(
			sourceHtml,
		)?.[1];
	if (retained !== undefined) return parseMarkdownSource(retained);
	const textBlock = /<script\s+type="text\/(?:markdown|plain)"\s*>([\s\S]*?)<\/script>/.exec(
		sourceHtml,
	);
	if (textBlock?.[1] === undefined)
		throw new Error('Island source requires an inert text block to preserve whitespace.');
	return { text: textBlock[1], reference: '' };
}

function readAuthorText(sourceHtml: string): string {
	return normalizeIslandText(readMarkdownRecord(sourceHtml).text);
}
export function prepareIslandContent(
	sourceHtml: string,
	diagrams: Readonly<Record<string, string>>,
	standalone: boolean,
): string {
	let preparedHtml = embedMarkdownFiles(sourceHtml, diagrams).replace(
		/<ex-mermaid([^>]*\bsrc="([^"]+)"[^>]*)>\s*<\/ex-mermaid>/g,
		(_match: string, attributes: string, sourcePath: string): string => {
			const sourceText = diagrams[sourcePath];
			if (!sourceText) throw new Error(`Unknown Mermaid island source: ${sourcePath}`);
			return `<ex-mermaid${attributes} prepared>${renderThemedDiagram(renderMermaidSVG, sourceText)}</ex-mermaid>`;
		},
	);
	if (!standalone) return preparedHtml;
	preparedHtml = preparedHtml.replace(
		/<ex-markdown([^>]*)>([\s\S]*?)<\/ex-markdown\s*>/g,
		(_match: string, attributes: string, content: string): string =>
			`<ex-markdown${attributes} prepared>${renderMarkdownContent(readAuthorText(content))}${markdownSourceMarkup(readMarkdownRecord(content))}</ex-markdown>`,
	);
	return preparedHtml.replace(
		/<ex-mermaid([^>]*)>([\s\S]*?)<\/ex-mermaid\s*>/g,
		(_match: string, attributes: string, content: string): string => {
			if (/\bprepared\b/.test(attributes)) return _match;
			const templateText = /<template>([\s\S]*?)<\/template>/.exec(content)?.[1];
			const sourceText =
				templateText === undefined
					? readAuthorText(content)
					: normalizeIslandText(decodeEscapedHtmlText(templateText));
			return `<ex-mermaid${attributes} prepared>${renderThemedDiagram(renderMermaidSVG, sourceText)}</ex-mermaid>`;
		},
	);
}
const fontFamilies = [
	{ slug: 'ibm-plex-sans-condensed', family: 'IBM Plex Sans Condensed', weights: [500, 600, 700] },
	{ slug: 'literata', family: 'Literata', weights: [400, 600] },
	{ slug: 'ibm-plex-mono', family: 'IBM Plex Mono', weights: [400] },
] as const;

/** Icons the theme ships: library controls, the tone vocabulary, and the few the lab pages name. */
export const shippedIconNames = [
	...subjectIcons,
	'caret-down',
	'arrow-counter-clockwise',
	'arrow-right',
	'arrow-left',
	'arrow-up-right',
	'arrows-out',
	'article',
	'caret-right',
	'changes',
	'check-circle',
	'circle-dashed',
	'circle-half',
	'controls',
	'corners-out',
	'diff',
	'info',
	'lightning',
	'list',
	'magnifying-glass-minus',
	'map-trifold',
	'magnifying-glass-plus',
	'minus-circle',
	'panel',
	'pencil-simple',
	'play',
	'plus-circle',
	'question',
	'sidebar-simple',
	'signpost',
	'warning',
	'x',
] as const;

export interface FontFile {
	readonly fileName: string;
	readonly sourceUrl: URL;
	readonly family: string;
	readonly weight: number;
}

export function localFontFiles(): readonly FontFile[] {
	return fontFamilies.flatMap((family) =>
		family.weights.map((weight): FontFile => ({
			fileName: `${family.slug}-${weight}.woff2`,
			sourceUrl: new URL(
				import.meta.resolve(
					`@fontsource/${family.slug}/files/${family.slug}-latin-${weight}-normal.woff2`,
				),
			),
			family: family.family,
			weight,
		})),
	);
}

function fontFaceRule(font: FontFile, source: string): string {
	return `@font-face{font-family:'${font.family}';font-style:normal;font-weight:${font.weight};font-display:swap;src:url(${source}) format('woff2')}`;
}

/** Font faces that reference separate files, so text paints before fonts arrive and unused faces never download. */
export function linkedFontStyles(directory: string): string {
	return localFontFiles()
		.map((font): string => fontFaceRule(font, `'${directory}/${font.fileName}'`))
		.join('\n');
}

/** Font faces embedded as data URLs for single-file offline bundles. */
export async function localFontStyles(): Promise<string> {
	return (
		await Promise.all(
			localFontFiles().map(async (font): Promise<string> =>
				fontFaceRule(
					font,
					`data:font/woff2;base64,${(await readFile(font.sourceUrl)).toString('base64')}`,
				),
			),
		)
	).join('\n');
}

export async function localPhosphorStyles(fontUrl: string): Promise<string> {
	const iconCss = await readFile(
		new URL(import.meta.resolve('@phosphor-icons/web/regular')),
		'utf8',
	);
	const shipped = new Set<string>(shippedIconNames);
	return iconCss
		.replace(
			/@font-face\s*\{[^}]*\}/,
			`@font-face{font-family:'Phosphor';font-style:normal;font-weight:normal;font-display:block;src:url('${fontUrl}') format('woff2')}`,
		)
		.replace(
			/\.ph\.ph-([a-z0-9-]+):before\s*\{[^}]*\}\s*/g,
			(rule: string, name: string): string => (shipped.has(name) ? rule : ''),
		);
}

/** Unicode code points of the shipped icons, for subsetting the icon font. */
export async function shippedIconCodePoints(): Promise<string> {
	const iconCss = await readFile(
		new URL(import.meta.resolve('@phosphor-icons/web/regular')),
		'utf8',
	);
	const shipped = new Set<string>(shippedIconNames);
	return Array.from(
		iconCss.matchAll(/\.ph\.ph-([a-z0-9-]+):before\s*\{\s*content:\s*"\\([0-9a-f]+)"/g),
	)
		.filter((match): boolean => shipped.has(match[1] ?? ''))
		.map((match): string => String.fromCodePoint(Number.parseInt(match[2] ?? '0', 16)))
		.join('');
}

export { bundleStandaloneHtml } from './standalone.ts';
export type { StandaloneHtmlBundle, StandaloneHtmlOptions } from './standalone.ts';
