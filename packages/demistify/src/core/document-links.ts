/** A destination is local only when it identifies this document and a valid nonempty fragment. */
export type DocumentLink =
	| { readonly kind: 'local'; readonly url: URL; readonly reference: string }
	| { readonly kind: 'other'; readonly url: URL }
	| { readonly kind: 'invalid'; readonly reason: 'url' | 'fragment' };

export function classifyDocumentLink(
	href: string,
	documentUrl: URL,
	baseUrl: URL = documentUrl,
): DocumentLink {
	let destination: URL;
	try {
		destination = new URL(href, baseUrl);
	} catch {
		return { kind: 'invalid', reason: 'url' };
	}
	if (
		destination.origin !== documentUrl.origin ||
		destination.pathname !== documentUrl.pathname ||
		destination.search !== documentUrl.search ||
		!destination.hash
	)
		return { kind: 'other', url: destination };
	try {
		return {
			kind: 'local',
			url: destination,
			reference: decodeURIComponent(destination.hash.slice(1)),
		};
	} catch {
		return { kind: 'invalid', reason: 'fragment' };
	}
}
