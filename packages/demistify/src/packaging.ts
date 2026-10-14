import { readFile } from 'node:fs/promises';

import { renderMermaidSVG } from 'beautiful-mermaid';

import { decodeEscapedHtmlText } from './html-text.ts';
import { renderMarkdownContent } from './markdown-content.ts';
import { normalizeIslandText } from './source-text.ts';
import { removeSvgFontImports } from './svg-fonts.ts';

function readAuthorText(sourceHtml: string): string {
	const textBlock = /<script\s+type="text\/(?:markdown|plain)"\s*>([\s\S]*?)<\/script>/.exec(
		sourceHtml,
	);
	if (textBlock?.[1] === undefined)
		throw new Error('Island source requires an inert text block to preserve whitespace.');
	return normalizeIslandText(textBlock[1]);
}
export function prepareIslandContent(
	sourceHtml: string,
	diagrams: Readonly<Record<string, string>>,
	standalone: boolean,
): string {
	let preparedHtml = sourceHtml.replace(
		/<ex-mermaid([^>]*\bsrc="([^"]+)"[^>]*)>\s*<\/ex-mermaid>/g,
		(_match: string, attributes: string, sourcePath: string): string => {
			const sourceText = diagrams[sourcePath];
			if (!sourceText) throw new Error(`Unknown Mermaid island source: ${sourcePath}`);
			return `<ex-mermaid${attributes} prepared>${removeSvgFontImports(renderMermaidSVG(sourceText, { bg: 'var(--sheet)', fg: 'var(--ink)', accent: 'var(--accent)', surface: 'var(--accent-soft)', border: 'var(--rule)', font: 'IBM Plex Sans Condensed', transparent: true }))}</ex-mermaid>`;
		},
	);
	if (!standalone) return preparedHtml;
	preparedHtml = preparedHtml.replace(
		/<ex-markdown([^>]*)>([\s\S]*?)<\/ex-markdown\s*>/g,
		(_match: string, attributes: string, content: string): string =>
			`<ex-markdown${attributes} prepared>${renderMarkdownContent(readAuthorText(content))}</ex-markdown>`,
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
			return `<ex-mermaid${attributes} prepared>${removeSvgFontImports(renderMermaidSVG(sourceText, { bg: 'var(--sheet)', fg: 'var(--ink)', accent: 'var(--accent)', surface: 'var(--accent-soft)', border: 'var(--rule)', font: 'IBM Plex Sans Condensed', transparent: true }))}</ex-mermaid>`;
		},
	);
}
export async function localFontStyles(): Promise<string> {
	const families = [
		{
			slug: 'ibm-plex-sans-condensed',
			family: 'IBM Plex Sans Condensed',
			weights: [500, 600, 700],
		},
		{ slug: 'literata', family: 'Literata', weights: [400, 500, 600] },
		{ slug: 'ibm-plex-mono', family: 'IBM Plex Mono', weights: [400, 500] },
	];
	return (
		await Promise.all(
			families.flatMap((family) =>
				family.weights.map(async (weight): Promise<string> => {
					const fontBytes = await readFile(
						new URL(
							import.meta.resolve(
								`@fontsource/${family.slug}/files/${family.slug}-latin-${weight}-normal.woff2`,
							),
						),
					);
					return `@font-face{font-family:'${family.family}';font-style:normal;font-weight:${weight};font-display:swap;src:url(data:font/woff2;base64,${fontBytes.toString('base64')}) format('woff2')}`;
				}),
			),
		)
	).join('\n');
}

export async function localPhosphorStyles(fontUrl: string): Promise<string> {
	const iconCss = await readFile(
		new URL(import.meta.resolve('@phosphor-icons/web/regular')),
		'utf8',
	);
	return iconCss.replace(
		/@font-face\s*\{[^}]*\}/,
		`@font-face{font-family:'Phosphor';font-style:normal;font-weight:normal;font-display:block;src:url('${fontUrl}') format('woff2')}`,
	);
}
