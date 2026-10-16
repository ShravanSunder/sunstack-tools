import { expect, test } from 'vitest';

import { parseGalleryFacetDeclaration } from './gallery-data.ts';

test('custom gallery facets use authored names and curated icons', (): void => {
	const facets = parseGalleryFacetDeclaration(
		'[{"field":"release","label":"Release","icon":"package"},{"field":"viewport"}]',
	);
	expect(facets).toEqual([
		{ field: 'release', label: 'Release', icon: 'package' },
		{ field: 'viewport', label: 'viewport', icon: '' },
	]);
});

test('invalid gallery metadata declarations fail at the input boundary', (): void => {
	expect((): unknown => parseGalleryFacetDeclaration('{"field":"release"}')).toThrow();
	expect((): unknown => parseGalleryFacetDeclaration('[{"field":"release name"}]')).toThrow();
	expect((): unknown =>
		parseGalleryFacetDeclaration('[{"field":"release","icon":"invented"}]'),
	).toThrow();
	expect((): unknown => parseGalleryFacetDeclaration('[{"field":"release","label":42}]')).toThrow();
});
