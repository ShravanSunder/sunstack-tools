/** Independent semantic assertions for real reports; no engine result is computed here. */
import assert from 'node:assert/strict';

import { z } from 'zod';

export function record(value: unknown): Record<string, unknown> {
	return z.record(z.string(), z.unknown()).parse(value);
}
export function rows(value: unknown): Record<string, unknown>[] {
	return z.array(z.record(z.string(), z.unknown())).parse(value);
}
export function text(value: unknown): string {
	return z.string().parse(value);
}
export function integer(value: unknown): bigint {
	if (typeof value === 'bigint') return value;
	assert.ok(typeof value === 'number' && Number.isSafeInteger(value), 'Expected exact integer');
	return BigInt(value);
}
export function stable(value: unknown): string {
	if (typeof value === 'bigint') return `bigint:${value}`;
	if (Array.isArray(value)) return `[${value.map(stable).join(',')}]`;
	if (value !== null && typeof value === 'object')
		return `{${Object.entries(value)
			.toSorted(([left], [right]) => left.localeCompare(right))
			.map(([key, item]) => `${JSON.stringify(key)}:${stable(item)}`)
			.join(',')}}`;
	return JSON.stringify(value) ?? 'undefined';
}
function withoutRunId(value: Record<string, unknown>): Record<string, unknown> {
	return Object.fromEntries(Object.entries(value).filter(([key]) => key !== 'id'));
}
function multiset(values: readonly unknown[]): string[] {
	return values.map(stable).toSorted();
}
export function status(report: Record<string, unknown>): string {
	return text(record(report['status'])['kind']);
}

/** Only run-local result IDs may differ. Keep every other field and duplicate. */
export function requireLocalParity(options: {
	readonly mechanical: unknown;
	readonly mixed: unknown;
	readonly names: readonly string[];
	readonly expectedFindings: number;
	readonly expectedSha256: string;
}): void {
	const mechanical = record(options.mechanical);
	const mixed = record(options.mixed);
	const selected = new Set(options.names);
	assert.equal(selected.size, options.names.length, 'Duplicate expected local owner');
	assert.ok(selected.size > 0, 'No independent local witness');
	const localResults = (report: Record<string, unknown>): Record<string, unknown>[] =>
		rows(report['results']).filter((row) => selected.has(text(row['name'])));
	const baseline = localResults(mechanical);
	assert.deepEqual(baseline.map((row) => text(row['name'])).toSorted(), [...selected].toSorted());
	assert.equal(rows(mechanical['findings']).length, options.expectedFindings);
	for (const result of baseline) {
		assert.equal(result['outcome'], 'finding', 'Authored local fixture must actually find');
		assert.equal(
			text(result['text_sha256']),
			options.expectedSha256,
			'Wrong original source digest',
		);
	}
	assert.deepEqual(
		multiset(localResults(mixed).map(withoutRunId)),
		multiset(baseline.map(withoutRunId)),
		'Independent result identity/content/multiplicity changed',
	);
	const localFindings = (report: Record<string, unknown>): Record<string, unknown>[] =>
		rows(report['findings']).filter((row) => selected.has(text(row['rule'])));
	assert.equal(localFindings(mixed).length, options.expectedFindings);
	assert.deepEqual(
		multiset(localFindings(mixed).map(withoutRunId)),
		multiset(localFindings(mechanical).map(withoutRunId)),
		'Independent finding content changed',
	);
}

export function requireUnavailableResults(
	reportValue: unknown,
	expected: ReadonlyMap<string, string>,
): void {
	const report = record(reportValue);
	const unavailable = rows(report['results']).filter((row) => row['outcome'] === 'unavailable');
	assert.deepEqual(
		unavailable.map((row) => text(row['name'])).toSorted(),
		[...expected.keys()].toSorted(),
		'Unavailable subjects dropped, duplicated or invented',
	);
	for (const row of unavailable) {
		const reason = record(row['reason']);
		assert.equal(reason['kind'], expected.get(text(row['name'])));
		switch (text(reason['kind'])) {
			case 'credential_unavailable':
				break;
			case 'filter_unavailable':
				text(reason['filter']);
				text(reason['folder']);
				break;
			case 'selection_unavailable':
				text(reason['evidence_kind']);
				text(reason['file']);
				break;
			case 'selected_field_unavailable':
				text(reason['field']);
				break;
			default:
				throw new Error('Unknown unavailable reason');
		}
		assert.ok(integer(row['id']) > 0n, 'Known unavailable result has no actual identity');
		assert.match(text(row['text_sha256']), /^[a-f0-9]{64}$/u);
		assert.deepEqual(row['request_ids'], [], 'Unavailable result invented request IDs');
		assert.deepEqual(row['answers'] ?? {}, {}, 'Unavailable result invented answers');
		assert.ok(
			row['basis'] === undefined || row['basis'] === null,
			'Unavailable result invented basis',
		);
	}
}

/** New SDK counts are derived from canonical facts, in distinct units. */
export function requireUnavailableCounts(reportValue: unknown): void {
	const report = record(reportValue);
	const counts = record(report['unavailable_counts']);
	const expected = {
		rule_pieces: rows(report['results']).filter((row) => row['outcome'] === 'unavailable').length,
		filter_folders: rows(report['filter_answers']).filter((row) => row['state'] === 'unavailable')
			.length,
		selection_candidates: rows(report['selections']).filter(
			(row) => record(row['outcome'])['kind'] === 'unavailable',
		).length,
		evidence_scopes: rows(report['incomplete']).filter(
			(row) => record(row['reason'])['kind'] === 'selection_unavailable',
		).length,
	};
	for (const [name, value] of Object.entries(expected))
		assert.equal(integer(counts[name]), BigInt(value), `Forged ${name} summary`);
}

export function requireZeroDispatch(reportValue: unknown, captures: readonly unknown[]): void {
	const report = record(reportValue);
	assert.equal(captures.length, 0, 'Keyless work submitted HTTP');
	assert.deepEqual(report['requests'], [], 'Keyless work invented a saved request');
	const usage = record(report['usage']);
	assert.equal(integer(usage['model_requests']), 0n, 'Keyless work consumed model requests');
	for (const name of ['input_tokens', 'output_tokens'])
		assert.ok(usage[name] === null || integer(usage[name]) === 0n, `Keyless work invented ${name}`);
	assert.ok(
		usage['cost_usd'] === null || usage['cost_usd'] === 0,
		'Keyless work invented a charge',
	);
	for (const name of ['cost_reported_requests', 'cost_missing_requests'])
		assert.equal(integer(usage[name]), 0n, 'Keyless work invented receipt accounting');
}

export function requireUnavailableSelection(reportValue: unknown): void {
	const report = record(reportValue);
	const selections = rows(report['selections']);
	assert.equal(selections.length, 1, 'Authored fixture has exactly one selected candidate');
	assert.equal(record(selections[0]?.['outcome'])['kind'], 'unavailable');
	assert.equal(
		record(record(selections[0]?.['outcome'])['reason'])['kind'],
		'credential_unavailable',
	);
	const counts = rows(report['file_counts']).flatMap((file) => {
		const selected = record(file['kinds'])['selected'];
		return selected === undefined ? [] : [record(record(selected)['selection'])];
	});
	assert.equal(counts.length, 1);
	const selected = counts[0];
	assert.ok(selected);
	assert.equal(integer(selected['candidates']), 1n);
	assert.equal(integer(selected['unavailable']), 1n);
	for (const name of ['asked', 'selected', 'not_selected', 'uncertain', 'incomplete'])
		assert.equal(integer(selected[name]), 0n, `Unavailable candidate misclassified as ${name}`);
	const scopes = rows(report['incomplete']).filter(
		(row) => record(row['reason'])['kind'] === 'selection_unavailable',
	);
	assert.ok(scopes.length > 0, 'No blocked selection coverage');
	const identities = scopes.map((row) => {
		const reason = record(row['reason']);
		return stable([reason['evidence_kind'], reason['file']]);
	});
	assert.equal(new Set(identities).size, identities.length, 'Duplicate kind/file gap');
}

export function requireProviderFailure(
	reportValue: unknown,
	httpStatus: number,
	captures: number,
): void {
	const report = record(reportValue);
	const expected = new Map([
		[400, 'provider_rejected_request'],
		[401, 'authentication'],
		[503, 'provider_unavailable'],
	]).get(httpStatus);
	assert.ok(expected && captures > 0, 'Failure has no real HTTP witness');
	const requests = rows(report['requests']);
	assert.ok(requests.length > 0);
	let attempts = 0;
	for (const request of requests) {
		assert.equal(request['state'], 'run');
		const outcome = record(request['outcome']);
		assert.equal(outcome['kind'], 'provider_failure');
		assert.equal(outcome['failure_class'], expected);
		assert.equal(outcome['http_status'], httpStatus);
		const records = rows(outcome['attempts']);
		assert.ok(records.length > 0, 'Submitted failure lost attempts');
		attempts += records.length;
		for (const attempt of records) {
			const evidence = record(attempt['evidence']);
			assert.equal(evidence['kind'], 'remote_outcome_unknown');
			assert.equal(evidence['http_status'], httpStatus);
		}
	}
	assert.equal(attempts, captures, 'Actual attempts differ from HTTP observations');
}

/** Keep exact selected IDs and actual links; unknown children must have no link. */
export function requireCaseLinks(options: {
	readonly cases: unknown;
	readonly expected: ReadonlyMap<string, { readonly kind: string; readonly linked: boolean }>;
	readonly actualResultIds: readonly unknown[];
}): void {
	const actual = rows(options.cases);
	assert.deepEqual(
		actual.map((row) => text(row['id'])).toSorted(),
		[...options.expected.keys()].toSorted(),
		'Selected case denominator changed',
	);
	const ids = new Set(options.actualResultIds.map((id) => integer(id).toString()));
	for (const row of actual) {
		const expected = options.expected.get(text(row['id']));
		assert.ok(expected);
		assert.equal(record(row['outcome'])['kind'], expected.kind);
		if (expected.linked) {
			assert.ok(row['result_id'] !== null && row['result_id'] !== undefined);
			assert.ok(ids.has(integer(row['result_id']).toString()), 'Case points to invented result');
		} else assert.equal(row['result_id'], null, 'Unknown child received an invented link');
	}
}

/** Original payload bytes, including model and every field, decide keyed parity. */
export function requireRequestParity(
	reference: readonly string[],
	candidate: readonly string[],
): void {
	assert.ok(reference.length > 0, 'No keyed HTTP witness');
	assert.deepEqual([...candidate].toSorted(), [...reference].toSorted(), 'Full HTTP bytes changed');
}
