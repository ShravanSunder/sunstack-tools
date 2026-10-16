/** Exact source retained separately from rendered HTML, including through packaging and inspection. */
export interface MarkdownSource {
	readonly text: string;
	readonly reference: string;
}

export function parseMarkdownSource(serialized: string): MarkdownSource {
	const value: unknown = JSON.parse(serialized);
	if (
		typeof value !== 'object' ||
		value === null ||
		!('text' in value) ||
		typeof value.text !== 'string' ||
		!('reference' in value) ||
		typeof value.reference !== 'string'
	)
		throw new Error('Invalid embedded Markdown source.');
	return { text: value.text, reference: value.reference };
}

export function markdownSourceMarkup(source: MarkdownSource): string {
	return `<script type="application/json" data-ex-markdown-source>${JSON.stringify(source).replaceAll('<', '\\u003c')}</script>`;
}
