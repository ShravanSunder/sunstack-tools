import { expect, test } from 'vitest';

import { type JsonObject } from '../rulebook/rulebook-fixture-support.ts';
import { requireUnionCounts } from './within-kinds-journey.ts';

const expected = [{ file: 'fixture.md', matches: 4, pieces: 3, picked: 3 }];
function report(): JsonObject {
	return {
		pieces: [{ text: 'Step only' }, { text: 'Standards only' }, { text: 'Both' }],
		file_counts: [
			{ file: 'fixture.md', kinds: { sentence: { matches: 4, pieces: 3, picked: 3 } } },
		],
	};
}

test('union counter assertion accepts literal fixture counts', () => {
	expect(() => requireUnionCounts({ report: report(), expected })).not.toThrow();
});

test('counter-only corruption fails while the correct piece list stays identical', () => {
	for (const counters of [
		{ matches: 3, pieces: 3, picked: 3 },
		{ matches: 4, pieces: 4, picked: 3 },
		{ matches: 4, pieces: 3, picked: 4 },
	]) {
		const corrupted = report();
		corrupted['file_counts'] = [{ file: 'fixture.md', kinds: { sentence: counters } }];
		expect(corrupted['pieces']).toEqual(report()['pieces']);
		expect(() => requireUnionCounts({ report: corrupted, expected })).toThrow(
			'Union counters must match fixture facts',
		);
	}
});
