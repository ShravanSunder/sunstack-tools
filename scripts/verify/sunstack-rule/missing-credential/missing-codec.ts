/** O3: keep every exact fact; mutate one original JSON span for each refusal. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import { jsonSlice } from '../prepare/raw-json.ts';
import { runSequence } from '../rulebook/rulebook-fixture-support.ts';
import { boundedSdkWait } from '../sdk/sdk-controlled-backend.ts';
import { requireDeepFrozenSdkValue } from '../sdk/sdk-journeys.ts';
import { saveSdkEvidence, type SdkExports, type SdkTarget } from '../sdk/sdk-target.ts';
import { record, requireUnavailableCounts, rows, stable, text } from './missing-checks.ts';
import { type SdkQualification } from './missing-sdk.ts';

export interface SavedSdkReport {
	readonly name: string;
	readonly json: string;
	readonly report: Record<string, unknown>;
}
export function saveMissingSdk(
	sdk: SdkExports,
	qualification: SdkQualification,
	evidence: string,
): SavedSdkReport[] {
	assert.ok(sdk.serialize);
	const serialize = sdk.serialize;
	return qualification.reports.map(({ name, report }) => {
		const saved = record(serialize({ kind: 'report', value: report }));
		assert.equal(saved['kind'], 'serialized');
		const json = text(saved['json']);
		filesystem.writeFileSync(path.join(evidence, `sdk-document-${name}.json`), json, {
			flag: 'wx',
		});
		return { name, json, report };
	});
}
function replaceSpan(
	json: string,
	location: readonly (string | number)[],
	replacement: string,
): string {
	const original = jsonSlice(json, location);
	assert.equal(json.split(original).length, 2, 'Mutation span must be unique');
	assert.notEqual(original, replacement, 'Mutation did not change its input');
	return json.replace(original, replacement);
}
export async function readDeletedSdk(options: {
	readonly sdk: SdkExports;
	readonly saved: readonly SavedSdkReport[];
	readonly target: SdkTarget;
	readonly evidence: string;
}): Promise<void> {
	const { sdk, saved, target, evidence } = options;
	assert.ok(sdk.deserialize && sdk.serialize);
	const deserialize = sdk.deserialize;
	await runSequence(saved, async (document) => {
		const readback = record(
			await boundedSdkWait(deserialize(document.json), `deleted ${document.name} readback`),
		);
		saveSdkEvidence(path.join(evidence, `readback-${document.name}.json`), readback);
		assert.equal(readback['kind'], 'decoded');
		const value = record(readback['value']);
		assert.equal(value['kind'], 'report');
		assert.equal(stable(value['value']), stable(document.report), 'SDK saved facts changed');
		requireUnavailableCounts(value['value']);
		requireDeepFrozenSdkValue(value['value']);
		const index = rows(document.report['results']).findIndex(
			(row) => row['outcome'] === 'unavailable',
		);
		if (index < 0) {
			assert.ok(
				rows(document.report['incomplete']).some(
					(row) => record(row['reason'])['kind'] === 'selection_unavailable',
				),
				'No known result or unknown-child scope witness',
			);
			const forged = replaceSpan(
				document.json,
				['payload', 'value', 'unavailable_counts'],
				'{"rule_pieces":0,"filter_folders":0,"selection_candidates":0,"evidence_scopes":0}',
			);
			const rejected = record(await deserialize(forged));
			saveSdkEvidence(path.join(evidence, `${document.name}-forged-scope-zero.json`), {
				input: forged,
				outcome: rejected,
			});
			assert.equal(rejected['kind'], 'invalid');
			const bindings = rows(record(document.report['case_selection'])['bindings']);
			assert.equal(bindings.length, 1);
			const malformed = replaceSpan(
				document.json,
				['payload', 'value', 'case_selection', 'bindings'],
				'[{"binding":' +
					jsonSlice(document.json, [
						'payload',
						'value',
						'case_selection',
						'bindings',
						0,
						'binding',
					]) +
					',"outcome":{"kind":"unavailable","reason":{"kind":"selection_unavailable"}}}]',
			);
			const invalid = record(await deserialize(malformed));
			saveSdkEvidence(path.join(evidence, `${document.name}-malformed-selection-reason.json`), {
				input: malformed,
				outcome: invalid,
			});
			assert.equal(invalid['kind'], 'invalid');
			return;
		}
		const location = ['payload', 'value', 'results', index] as const;
		const originalResult = record(JSON.parse(jsonSlice(document.json, location)));
		const noReason = { ...originalResult };
		delete noReason['reason'];
		const mutations = [
			{
				name: 'missing-reason',
				json: replaceSpan(document.json, location, JSON.stringify(noReason)),
			},
			{
				name: 'wrong-reason',
				json: replaceSpan(
					document.json,
					location,
					JSON.stringify({ ...originalResult, reason: { kind: 'filter_unavailable' } }),
				),
			},
			{
				name: 'unknown-ending',
				json: replaceSpan(
					document.json,
					location,
					JSON.stringify({ ...originalResult, outcome: 'invented' }),
				),
			},
			{
				name: 'unknown-reason',
				json: replaceSpan(
					document.json,
					location,
					JSON.stringify({ ...originalResult, reason: { kind: 'invented' } }),
				),
			},
			{
				name: 'credential-reason-extra-payload',
				json: replaceSpan(
					document.json,
					location,
					JSON.stringify({
						...originalResult,
						reason: { kind: 'credential_unavailable', extra: true },
					}),
				),
			},
			{
				name: 'forged-zero',
				json: replaceSpan(
					document.json,
					['payload', 'value', 'unavailable_counts'],
					'{"rule_pieces":0,"filter_folders":0,"selection_candidates":0,"evidence_scopes":0}',
				),
			},
			{
				name: 'overflow',
				json: replaceSpan(
					document.json,
					location,
					JSON.stringify(originalResult).replace(/"id":\d+/u, '"id":18446744073709551616'),
				),
			},
		];
		for (const [resultIndex, result] of rows(document.report['results']).entries()) {
			if (result['outcome'] !== 'unavailable' || resultIndex === index) continue;
			const original = record(
				JSON.parse(jsonSlice(document.json, ['payload', 'value', 'results', resultIndex])),
			);
			const missing = { ...original };
			delete missing['reason'];
			mutations.push({
				name: `result-${resultIndex}-missing-reason`,
				json: replaceSpan(
					document.json,
					['payload', 'value', 'results', resultIndex],
					JSON.stringify(missing),
				),
			});
			const kind = text(record(result['reason'])['kind']);
			if (kind !== 'credential_unavailable')
				mutations.push({
					name: `result-${resultIndex}-missing-${kind}-payload`,
					json: replaceSpan(
						document.json,
						['payload', 'value', 'results', resultIndex],
						JSON.stringify({ ...original, reason: { kind } }),
					),
				});
		}
		await runSequence(mutations, async (mutation) => {
			const outcome = record(await boundedSdkWait(deserialize(mutation.json), mutation.name));
			saveSdkEvidence(path.join(evidence, `${document.name}-${mutation.name}.json`), {
				input: mutation.json,
				outcome,
			});
			assert.equal(outcome['kind'], 'invalid', `${mutation.name}: malformed SDK value accepted`);
		});
		const wrongRevision = document.json.replace(
			/"contract_revision"\s*:\s*\d+/u,
			`"contract_revision":${target.contractRevision + 1}`,
		);
		assert.notEqual(wrongRevision, document.json);
		const incompatible = record(await deserialize(wrongRevision));
		saveSdkEvidence(path.join(evidence, `${document.name}-revision.json`), incompatible);
		assert.equal(incompatible['kind'], 'incompatible');
		assert.equal(incompatible['expected_contract_revision'], target.contractRevision);
		assert.equal(incompatible['actual_contract_revision'], target.contractRevision + 1);
	});
	// A legitimate configuration reference carries wide integers without inventing runtime counts.
	const source = `{"contract_revision":${target.contractRevision},"payload":{"kind":"configuration","value":{"id":"wide-reference","base_directory":"/nonexistent-definitions","marker_prefix":"K","check":{"files":["*.md"]},"decision_model":{"model":"jev-latest"},"limits":{"max_file_bytes":18446744073709551615},"rulebooks":[{"kind":"files","markdown":"missing.md","entries":"missing.json"}],"evidence":{"kind":"project","config_path":"missing.yml","kinds":{}}}}}`;
	const wide = record(await deserialize(source));
	assert.equal(wide['kind'], 'decoded');
	const configuration = record(record(wide['value'])['value']);
	assert.equal(record(configuration['limits'])['max_file_bytes'], 18446744073709551615n);
	const encoded = record(sdk.serialize(wide['value']));
	assert.equal(encoded['kind'], 'serialized');
	assert.ok(text(encoded['json']).includes('18446744073709551615'));
	saveSdkEvidence(path.join(evidence, 'wide-reference.json'), { source, wide, encoded });
}
