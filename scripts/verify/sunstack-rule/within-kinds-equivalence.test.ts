import { expect, test } from 'vitest';

import { type JsonObject } from './rulebook-fixture-support.ts';
import { requireWithinEquivalence } from './within-kinds-equivalence.ts';

function listing(): JsonObject {
	return {
		pieces: [
			{
				kind: 'step',
				where: { first_line: 7 },
				fields: { body: { state: 'text', value: 'Literal guidance.' } },
			},
		],
		selections: [{ outcome: { kind: 'selected', yes: 0.93 } }],
		file_counts: [{ file: 'one.md', kinds: { step: { matches: 1, pieces: 1 } } }],
	};
}
test('W6 accepts identical fields and rejects a planted piece property without normalizing it', () => {
	const reference = listing();
	expect(() => requireWithinEquivalence({ reference, current: listing() })).not.toThrow();
	const planted = listing();
	planted['pieces'] = [
		{
			kind: 'step',
			where: { first_line: 8 },
			fields: { body: { state: 'text', value: 'Literal guidance.' } },
		},
	];
	expect(() => requireWithinEquivalence({ reference, current: planted })).toThrow(
		'W6 differs in pieces',
	);
});
test('W6 requires selections and counts as well as pieces', () => {
	for (const field of ['selections', 'file_counts']) {
		const current = listing();
		current[field] = [];
		expect(() => requireWithinEquivalence({ reference: listing(), current })).toThrow(
			'W6 differs in ' + field,
		);
	}
});
