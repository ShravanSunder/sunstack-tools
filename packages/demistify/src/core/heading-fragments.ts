/** Stable fragments shared by Markdown links and the document outline. */
export function headingSlug(label: string): string {
	return (
		label
			.trim()
			.toLowerCase()
			.replace(/[^\p{L}\p{N}]+/gu, '-')
			.replace(/^-|-$/g, '') || 'section'
	);
}

export function assignHeadingFragments(root: HTMLElement): void {
	const localFragments = new Map<string, string>();
	for (const heading of root.querySelectorAll<HTMLHeadingElement>('h1,h2,h3,h4,h5,h6')) {
		const slug = headingSlug(heading.textContent ?? '');
		if (!heading.id) {
			const base =
				root.localName === 'ex-markdown' && root.id ? `${root.id}--${slug}` : `heading-${slug}`;
			heading.id = base;
			let suffix = 1;
			while (document.getElementById(heading.id) !== heading) heading.id = `${base}-${++suffix}`;
		}
		if (!localFragments.has(slug)) localFragments.set(slug, heading.id);
	}
	for (const link of root.querySelectorAll<HTMLAnchorElement>('a[href^="#"]')) {
		const fragment = decodeURIComponent(link.hash.slice(1));
		// Markdown shorthand belongs to its island; page-wide links retain real authored IDs.
		if (root.localName !== 'ex-markdown' && document.getElementById(fragment)) continue;
		const target = localFragments.get(fragment);
		if (target) link.setAttribute('href', `#${target}`);
	}
}
