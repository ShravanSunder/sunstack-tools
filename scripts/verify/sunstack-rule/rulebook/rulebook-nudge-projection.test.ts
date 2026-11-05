import { expect, test } from 'vitest';

import { requireNudgeCompatibility } from './rulebook-nudge-projection.ts';

const baseline =
	'{"version":"0.5.0","results":[{"step":"gate","outcome":"pass"},{"step":"next","outcome":"finding"}],"findings":[{"line":7,"span":{"start_line":7}}],"evidence_seen":{"10":{"body":"ten"},"2":{"body":"two"}},"usage":{"model_requests":2}}';
const current =
	'{"version":"0.6.0","results":[{"step":"gate","outcome":"pass","path":["gate"],"basis":"judged"},{"step":"next","outcome":"finding","path":["next"]}],"findings":[{"line":7,"anchor":"default","span":{"start_line":7},"related":[]}],"evidence_seen":{"10":{"body":"ten"},"2":{"body":"two"}},"usage":{"model_requests":2},"new_field":true}';

test('U10 accepts additive fields at each depth and preserves numerical key emission order', () => {
	expect(() => requireNudgeCompatibility(baseline, current)).not.toThrow();
});
test.each([
	['wrong version', current.replace('0.6.0', '0.5.0')],
	['changed step', current.replace('"step":"gate"', '"step":"wrong"')],
	['changed outcome', current.replace('"outcome":"pass"', '"outcome":"finding"')],
	['changed location', current.replace('"line":7', '"line":17')],
	['changed count', current.replace('"model_requests":2', '"model_requests":3')],
	['renamed field', current.replace('"body":"ten"', '"renamed":"ten"')],
	[
		'reordered field',
		current.replace('"step":"gate","outcome":"pass"', '"outcome":"pass","step":"gate"'),
	],
	[
		'reordered integer keys',
		current.replace(
			'"10":{"body":"ten"},"2":{"body":"two"}',
			'"2":{"body":"two"},"10":{"body":"ten"}',
		),
	],
	[
		'reordered results',
		current
			.replace('"step":"gate"', '"step":"next"')
			.replace('"step":"next","outcome":"finding"', '"step":"gate","outcome":"finding"'),
	],
	['removed old member', current.replace('"model_requests":2', '')],
	['type changed', current.replace('"model_requests":2', '"model_requests":"2"')],
])('U10 rejects %s', (_name, changed) => {
	expect(() => requireNudgeCompatibility(baseline, changed)).toThrow();
});
test('U10 rejects duplicate legacy keys before JSON.parse can erase the first value', () => {
	expect(() =>
		requireNudgeCompatibility(baseline, current.replace('"line":7', '"line":999,"line":7')),
	).toThrow('Duplicate report key');
});
