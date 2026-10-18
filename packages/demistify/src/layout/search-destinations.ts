import { documentSearchIndexSchema, type SearchDestination } from '../core/document-search.ts';
import { assignHeadingFragments } from '../core/heading-fragments.ts';
import { reportIssue } from '../core/issues.ts';

/** A picture-only local path keeps the images glyph, including prepared gallery sources. */
function picturedPath(presentation: Element): boolean {
	const steps = Array.from(presentation.querySelectorAll(':scope > ex-step'));
	return (
		steps.length > 0 &&
		steps.every((step): boolean => {
			const reference = step.getAttribute('to') ?? '';
			let target = document.getElementById(reference);
			for (const source of document.querySelectorAll('ex-gallery > template[data-gallery-source]'))
				if (!target && source instanceof HTMLTemplateElement)
					target = source.content.getElementById(reference);
			return Boolean(
				target && (target.matches('img, figure, ex-gallery') || target.closest('ex-gallery')),
			);
		})
	);
}

export function readSearchIndex(): readonly SearchDestination[] {
	const carrier = document.querySelector('script[type="application/json"][data-ex-search-index]');
	if (!carrier) return [];
	try {
		return documentSearchIndexSchema.parse(JSON.parse(carrier.textContent ?? ''));
	} catch {
		reportIssue(carrier, 'Invalid document search index. Regenerate with demistify index.');
		return [];
	}
}

/** Live headings replace this page's generated entries after Markdown and other islands render. */
export function collectSearchDestinations(props: {
	readonly contentRoot: HTMLElement;
	readonly navigation: HTMLElement;
	readonly indexed: readonly SearchDestination[];
	readonly pageTitle: string;
}): readonly SearchDestination[] {
	const { contentRoot, navigation, indexed, pageTitle } = props;
	assignHeadingFragments(contentRoot);
	const here = new URL(location.href);
	here.hash = '';
	const entries = new Map<string, SearchDestination>();
	const add = (entry: SearchDestination): void => {
		const url = new URL(entry.href, document.baseURI);
		const key =
			entry.kind === 'page'
				? `${entry.kind}:${url.origin}${url.pathname}${url.search}`
				: `${entry.kind}:${url.href}`;
		entries.set(key, entry);
	};
	for (const link of navigation.querySelectorAll<HTMLAnchorElement>('a[href]')) {
		const title =
			link.querySelector('strong')?.textContent?.trim() ?? link.textContent?.trim() ?? '';
		if (!title) continue;
		const kind = link.closest('[slot="presentations"]') ? 'walkthrough' : 'page';
		const parsed = documentSearchIndexSchema.safeParse([
			{
				kind,
				title,
				pageTitle: title,
				ancestors: [],
				href: link.href,
				description: link.querySelector('span')?.textContent?.trim(),
			},
		]);
		if (parsed.success) for (const entry of parsed.data) add(entry);
	}
	for (const entry of indexed) {
		const url = new URL(entry.href, document.baseURI);
		url.hash = '';
		if (url.href !== here.href) add(entry);
	}
	add({ kind: 'page', title: pageTitle, pageTitle, ancestors: [], href: here.href });
	const parents: { level: number; title: string }[] = [];
	const firstHeading = contentRoot.querySelector('h1');
	for (const heading of contentRoot.querySelectorAll<HTMLHeadingElement>('h1,h2,h3,h4,h5,h6')) {
		if (heading === firstHeading || heading.closest('figcaption, ex-gallery, ex-presentation, svg'))
			continue;
		const title = heading.textContent?.trim() ?? '';
		if (!heading.id || !title) continue;
		const level = Number(heading.tagName.slice(1));
		while ((parents.at(-1)?.level ?? 0) >= level) parents.pop();
		add({
			kind: 'heading',
			title,
			pageTitle,
			ancestors: parents.map((parent): string => parent.title),
			href: `#${encodeURIComponent(heading.id)}`,
		});
		parents.push({ level, title });
	}
	for (const presentation of document.querySelectorAll('ex-presentation[id]'))
		add({
			kind: picturedPath(presentation) ? 'journey' : 'walkthrough',
			title: presentation.getAttribute('title') ?? presentation.id,
			pageTitle,
			ancestors: [],
			href: `#${encodeURIComponent(presentation.id)}`,
			description: presentation.getAttribute('description') ?? '',
		});
	return [...entries.values()];
}
