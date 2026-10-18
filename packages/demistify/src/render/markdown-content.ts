import { Marked } from 'marked';
import type { Tokens } from 'marked';

import { escapeHtml } from '../core/html-text.ts';
import { normalizeIslandText } from '../core/source-text.ts';

export function renderMarkdownContent(sourceText: string): string {
	const parser = new Marked({
		renderer: {
			html: (token: Tokens.HTML | Tokens.Tag): string => escapeHtml(token.text),
			code: (token: Tokens.Code): string | false => {
				if (token.lang?.trim().split(/\s+/)[0]?.toLowerCase() !== 'mermaid') return false;
				return `<ex-mermaid><template>${escapeHtml(token.text)}</template></ex-mermaid>`;
			},
		},
	});
	return parser
		.parse(normalizeIslandText(sourceText), { async: false })
		.replaceAll('<table>', '<div class="ex-table-scroll"><table>')
		.replaceAll('</table>', '</table></div>');
}

/** Focused narration shares Markdown parsing; large visuals remain in the walkthrough target. */
export function renderNarrationContent(sourceText: string, baseUrl: string): string {
	const parser = new Marked({
		renderer: {
			html: (token: Tokens.HTML | Tokens.Tag): string => escapeHtml(token.text),
			heading: (token: Tokens.Heading): string =>
				`<p><strong>${parser.parseInline(token.text, { async: false })}</strong></p>`,
			image: (token: Tokens.Image): string => escapeHtml(token.text),
			code: (token: Tokens.Code): string => `<p><code>${escapeHtml(token.text)}</code></p>`,
			table: (token: Tokens.Table): string =>
				`<p>${escapeHtml(token.raw).replaceAll('\n', '<br>')}</p>`,
			// oxlint-disable-next-line id-length -- Marked names this renderer for the HTML tag.
			hr: (): string => '',
			link: (token: Tokens.Link): string => {
				const text = parser.parseInline(token.text, { async: false });
				try {
					const destination = new URL(token.href, baseUrl);
					if (!['http:', 'https:', 'mailto:', 'file:'].includes(destination.protocol)) return text;
					const title = token.title ? ` title="${escapeHtml(token.title)}"` : '';
					return `<a href="${escapeHtml(destination.href)}"${title}>${text}</a>`;
				} catch {
					return text;
				}
			},
		},
	});
	return parser.parse(normalizeIslandText(sourceText), { async: false });
}
