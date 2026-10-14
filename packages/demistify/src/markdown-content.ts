import { Marked } from 'marked';
import type { Tokens } from 'marked';

import { escapeHtml } from './html-text.ts';
import { normalizeIslandText } from './source-text.ts';

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
