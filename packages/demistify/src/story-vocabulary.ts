import type { AttributeSpec, ElementSpec } from './vocabulary.ts';

/** The single named-child contract consumed by story guards, the checker and authoring metadata. */
export const storySlots = [
	{ name: 'headline', required: true, disclosure: false },
	{ name: 'happened', required: true, disclosure: false },
	{ name: 'for-you', required: true, disclosure: false },
	{ name: 'next', required: true, disclosure: false },
	{ name: 'showpiece', required: false, disclosure: false },
	{ name: 'more', required: false, disclosure: true },
	{ name: 'record', required: true, disclosure: true },
] as const;

interface StoryAttributes {
	readonly icon: AttributeSpec;
	readonly tone: AttributeSpec;
	readonly series: AttributeSpec;
}

/** Reuse the canonical shared attribute definitions without a runtime import cycle. */
export function storyElementVocabulary(
	shared: StoryAttributes,
): Readonly<Record<string, ElementSpec>> {
	return {
		'ex-map': {
			description:
				'One authored Tree in the shared non-modal Map rail, mutually exclusive with Comments.',
			attributes: {
				for: {
					description: 'Id of the original page scope whose sections the Map names.',
					type: { kind: 'id' },
					required: true,
				},
			},
		},
		'ex-story': {
			description:
				'One native story disclosure with a headline, labelled rows and supporting record.',
			attributes: {
				subject: {
					description: 'Supplied subject beside the story title.',
					type: { kind: 'text' },
				},
				time: { description: 'Supplied display time, preserved as text.', type: { kind: 'text' } },
				icon: shared.icon,
				tone: shared.tone,
			},
		},
		'ex-section': {
			description:
				'A story group with a sticky category header, derived counts and scoped fold control.',
			attributes: {
				id: {
					description: 'Stable section target for Map.',
					type: { kind: 'text' },
					required: true,
				},
				title: {
					description: 'The authored section name, without a count.',
					type: { kind: 'text' },
					required: true,
				},
				icon: shared.icon,
				tone: shared.tone,
				series: shared.series,
			},
		},
		'ex-fold-toggle': {
			description: 'An icon-only expand/collapse action over the named story disclosure scope.',
			attributes: {
				for: {
					description: 'Id of the element containing the disclosure scope.',
					type: { kind: 'id' },
					required: true,
				},
			},
		},
	};
}
