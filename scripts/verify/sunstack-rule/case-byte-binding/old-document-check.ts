/** Read genuine old-writer documents through the installed candidate native codec. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import { z } from 'zod';

import { boundedSdkWait } from '../sdk/sdk-controlled-backend.ts';
import { fileDigest, requireSdk, saveSdkEvidence, type SdkExports } from '../sdk/sdk-target.ts';

const outcomeSchema = z.object({ kind: z.string() }).passthrough();

export async function observeOldDocumentRefusals(options: {
	readonly sdk: SdkExports;
	readonly evidence: string;
	readonly documents: readonly {
		readonly path: string;
		readonly sha256: string;
		readonly revision: number;
	}[];
}): Promise<void> {
	requireSdk(options.sdk.deserialize, 'Installed native decoder unavailable');
	const deserialize = options.sdk.deserialize;
	const revisions = options.documents
		.map((item) => item.revision)
		.toSorted((left, right) => left - right);
	assert.deepEqual(revisions, [20, 21]);
	const currentCounterfactual = JSON.stringify({ contract_revision: 22, payload: null });
	const currentInvalid = outcomeSchema.parse(
		await boundedSdkWait(deserialize(currentCounterfactual), 'current revision invalid payload'),
	);
	assert.equal(
		currentInvalid.kind,
		'invalid',
		'Current invalid payload did not reach the value decoder',
	);
	saveSdkEvidence(path.join(options.evidence, 'current-invalid-payload.json'), {
		input: currentCounterfactual,
		outcome: currentInvalid,
	});
	const observe = async (document: (typeof options.documents)[number]): Promise<void> => {
		assert.ok(path.isAbsolute(document.path));
		assert.equal(fileDigest(document.path), document.sha256, 'Old document hash differs');
		const text = filesystem.readFileSync(document.path, 'utf8');
		const framing = z
			.object({ contract_revision: z.number().int(), payload: z.unknown() })
			.parse(JSON.parse(text));
		assert.equal(framing.contract_revision, document.revision);
		const original = outcomeSchema.parse(
			await boundedSdkWait(deserialize(text), `old revision ${document.revision}`),
		);
		const counterfactual = JSON.stringify({ contract_revision: document.revision, payload: null });
		const malformed = outcomeSchema.parse(
			await boundedSdkWait(
				deserialize(counterfactual),
				`old revision ${document.revision} invalid payload`,
			),
		);
		for (const outcome of [original, malformed]) {
			assert.equal(outcome.kind, 'incompatible');
			assert.equal(outcome['expected_contract_revision'], 22);
			assert.equal(outcome['actual_contract_revision'], document.revision);
		}
		saveSdkEvidence(path.join(options.evidence, `old-${document.revision}-refusal.json`), {
			inputPath: document.path,
			inputSha256: document.sha256,
			original,
			counterfactual,
			malformed,
		});
	};
	await observe(
		options.documents[0] ??
			(() => {
				throw new Error('Missing first old document');
			})(),
	);
	await observe(
		options.documents[1] ??
			(() => {
				throw new Error('Missing second old document');
			})(),
	);
}
