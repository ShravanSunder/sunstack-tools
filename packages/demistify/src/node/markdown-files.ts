import { type DefaultTreeAdapterMap, parse, serialize } from 'parse5';

import { markdownSourceMarkup } from '../core/markdown-source.ts';

type HtmlElement = DefaultTreeAdapterMap['element'];

function* markdownElements(parent: DefaultTreeAdapterMap['parentNode']): Generator<HtmlElement> {
	for (const child of parent.childNodes) {
		if (!('tagName' in child)) continue;
		if (
			child.tagName === 'ex-markdown' &&
			child.attrs.some((attribute): boolean => attribute.name === 'src')
		)
			yield child;
		yield* markdownElements('content' in child ? child.content : child);
	}
}

export function markdownFileReferences(html: string): readonly string[] {
	return Array.from(
		markdownElements(parse(html)),
		(element): string =>
			element.attrs.find((attribute): boolean => attribute.name === 'src')?.value ?? '',
	);
}

/** Resolve supplied files once at packaging time; no remote fetch, runtime or per-page application. */
export function embedMarkdownFiles(
	html: string,
	sources: Readonly<Record<string, string>>,
): string {
	if (!/<ex-markdown\b[^>]*\bsrc=/i.test(html)) return html;
	const document = parse(html);
	for (const element of markdownElements(document)) {
		const reference =
			element.attrs.find((attribute): boolean => attribute.name === 'src')?.value ?? '';
		const text = sources[reference];
		if (text === undefined) throw new Error(`Unknown Markdown island source: ${reference}`);
		const sourceDocument = parse(`<body>${markdownSourceMarkup({ text, reference })}</body>`);
		const htmlElement = sourceDocument.childNodes.find(
			(child): child is HtmlElement => 'tagName' in child && child.tagName === 'html',
		);
		const body = htmlElement?.childNodes.find(
			(child): child is HtmlElement => 'tagName' in child && child.tagName === 'body',
		);
		if (!body) throw new Error('Cannot embed Markdown source.');
		element.attrs = element.attrs.filter((attribute): boolean => attribute.name !== 'src');
		element.childNodes = body.childNodes;
		for (const child of element.childNodes) child.parentNode = element;
	}
	return serialize(document);
}
