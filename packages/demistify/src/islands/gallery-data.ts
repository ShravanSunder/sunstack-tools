import { z } from 'zod';

import { subjectIcons } from '../core/subject-icons.ts';

export interface GalleryCapture {
	readonly id: string;
	readonly title: string;
	readonly source: HTMLElement;
	readonly metadata: Readonly<Record<string, string>>;
	readonly searchText: string;
}

const galleryFacetSchema = z.object({
	field: z.string().regex(/^[a-z][a-z0-9-]*$/, 'Gallery filter field must name a data attribute.'),
	label: z.string(),
	icon: z.union([z.enum(subjectIcons), z.literal('')]),
});

const galleryFacetInputSchema = galleryFacetSchema.extend({
	label: galleryFacetSchema.shape.label.optional(),
	icon: galleryFacetSchema.shape.icon.default(''),
});

export type GalleryFacet = Readonly<z.infer<typeof galleryFacetSchema>>;

/** Parse authored JSON once; facet consumers receive only validated field and icon names. */
export function parseGalleryFacetDeclaration(sourceText: string): readonly GalleryFacet[] {
	return z
		.array(galleryFacetInputSchema)
		.parse(JSON.parse(sourceText))
		.map((entry): GalleryFacet => ({
			field: entry.field,
			label: entry.label ?? entry.field,
			icon: entry.icon,
		}));
}

export function readGalleryCaptures(owner: ParentNode, prefix: string): readonly GalleryCapture[] {
	const figures = Array.from(owner.children).filter(
		(child): child is HTMLElement => child instanceof HTMLElement && child.localName === 'figure',
	);
	return figures.map((figure, index): GalleryCapture => {
		const image = figure.querySelector<HTMLImageElement>('img');
		if (!image?.getAttribute('src')?.trim())
			throw new Error('Each gallery figure needs an img src.');
		const metadata = Object.fromEntries(
			Array.from(figure.attributes)
				.filter((attribute): boolean => attribute.name.startsWith('data-'))
				.map((attribute): readonly [string, string] => [attribute.name.slice(5), attribute.value]),
		);
		metadata['screen'] ||= 'Captures';
		const authoredTitle = figure.dataset['title'] ?? image.alt;
		const title = authoredTitle.trim() ? authoredTitle : `Capture ${index + 1}`;
		return {
			id: figure.id || `${prefix}--capture-${index + 1}`,
			title,
			source: figure,
			metadata,
			searchText: [title, image.getAttribute('src'), figure.textContent, ...Object.values(metadata)]
				.join(' ')
				.toLowerCase(),
		};
	});
}

export function readGalleryFacets(
	owner: HTMLElement,
	captures: readonly GalleryCapture[],
): readonly GalleryFacet[] {
	const declaration = owner.querySelector('script[data-gallery-filters]');
	const fields = owner.getAttribute('filters')?.split(/\s+/).filter(Boolean) ?? ['tags'];
	const defaults = fields.filter((field): boolean =>
		captures.some((capture): boolean => !!capture.metadata[field]),
	);
	if (!declaration)
		return defaults.map((field): GalleryFacet => ({
			field,
			label: field[0]?.toUpperCase() + field.slice(1),
			icon: '',
		}));
	return parseGalleryFacetDeclaration(declaration.textContent ?? 'null');
}

export function filterGalleryCaptures(
	captures: readonly GalleryCapture[],
	query: string,
	facets: ReadonlyMap<string, ReadonlySet<string>>,
): readonly GalleryCapture[] {
	const normalized = query.trim().toLowerCase();
	return captures.filter(
		(capture): boolean =>
			(!normalized || capture.searchText.includes(normalized)) &&
			Array.from(facets).every(
				([field, values]): boolean =>
					values.size === 0 ||
					galleryFacetValues(capture, field).some((value): boolean => values.has(value)),
			),
	);
}

/** Tags are comma-separated labels; ordinary metadata facets contain one value. */
export function galleryFacetValues(capture: GalleryCapture, field: string): readonly string[] {
	const value = capture.metadata[field] ?? '';
	return field === 'tags'
		? value
				.split(',')
				.map((tag): string => tag.trim())
				.filter(Boolean)
		: value
			? [value]
			: [];
}
/** A logical screen/state is one step; viewport captures are its variations. */
export function galleryScreenKey(capture: GalleryCapture): string {
	return JSON.stringify([
		capture.source.dataset['screen'] ?? capture.id,
		capture.metadata['state'] ?? '',
	]);
}
