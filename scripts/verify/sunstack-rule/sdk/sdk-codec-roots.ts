/** Roundtrip each supported root using values returned by the installed addon. */
import filesystem from 'node:fs';
import path from 'node:path';
import { isDeepStrictEqual } from 'node:util';

import { z } from 'zod';

import { boundedSdkWait, type SdkControlledBackend } from './sdk-controlled-backend.ts';
import {
	createCheckedSdkInputs,
	createSdkFileDefinitions,
	sdkInlineConfiguration,
} from './sdk-fixtures.ts';
import { requireDeepFrozenSdkValue, type JourneyResult } from './sdk-journeys.ts';
import { requireSdk, saveSdkEvidence, type SdkExports } from './sdk-target.ts';

const recordSchema = z.record(z.string(), z.unknown());

export async function observeSdkCodecRoots(options: {
	readonly sdk: SdkExports;
	readonly evidence: string;
	readonly backend: SdkControlledBackend;
}): Promise<JourneyResult> {
	const { sdk, evidence, backend } = options;
	requireSdk(
		sdk.serialize && sdk.deserialize && sdk.inspect,
		'Codec root journey requires installed public operations',
	);
	const serialize = sdk.serialize;
	const deserialize = sdk.deserialize;
	const root = path.join(evidence, 'codec-root-input');
	filesystem.mkdirSync(root);
	createCheckedSdkInputs(root);
	createSdkFileDefinitions(root, backend.endpoint);
	const loaded = recordSchema.parse(
		await boundedSdkWait(sdk.load({ directory: root }), 'codec load fixture'),
	);
	requireSdk(loaded['kind'] === 'loaded', 'Codec fixture did not load');
	const captured = recordSchema.parse(loaded['value']);
	const checked = recordSchema.parse(
		await boundedSdkWait(sdk.check(captured), 'codec check fixture'),
	);
	requireSdk(checked['kind'] === 'valid', 'Codec fixture did not admit');
	const cancellation = new AbortController();
	const pending = sdk.run(captured, { root, scope: { kind: 'all' }, signal: cancellation.signal });
	let run: Record<string, unknown>;
	try {
		run = recordSchema.parse(await boundedSdkWait(pending, 'codec run fixture'));
	} finally {
		cancellation.abort();
		await boundedSdkWait(Promise.allSettled([pending]), 'codec run settlement');
	}
	requireSdk(run['kind'] === 'completed', 'Codec fixture did not complete its real run');
	const inspection = recordSchema.parse(
		await boundedSdkWait(
			sdk.inspect(captured, { root, scope: { kind: 'all' } }),
			'codec inspection fixture',
		),
	);
	requireSdk(inspection['kind'] === 'inspected', 'Codec fixture did not complete inspection');
	// LoadedConfiguration.configuration is a captured ResolvedConfiguration, not the
	// authored Configuration root. Obtain a canonical authored root through the codec.
	const initialConfiguration = recordSchema.parse(
		serialize({ kind: 'configuration', value: sdkInlineConfiguration(root, backend.endpoint) }),
	);
	requireSdk(
		initialConfiguration['kind'] === 'serialized' &&
			typeof initialConfiguration['json'] === 'string',
		'Codec configuration fixture did not serialize',
	);
	const canonicalReply = recordSchema.parse(
		await boundedSdkWait(deserialize(initialConfiguration['json']), 'canonical configuration'),
	);
	requireSdk(canonicalReply['kind'] === 'decoded', 'Codec configuration fixture did not decode');
	const canonicalConfiguration = recordSchema.parse(canonicalReply['value']);
	requireSdk(
		canonicalConfiguration['kind'] === 'configuration',
		'Codec fixture decoded the wrong root',
	);
	const roots = [
		{ kind: 'configuration', value: canonicalConfiguration['value'] },
		{ kind: 'loaded_configuration', value: captured },
		{ kind: 'load_outcome', value: loaded },
		{ kind: 'check_outcome', value: checked },
		{ kind: 'run_outcome', value: run },
		{ kind: 'inspection_outcome', value: inspection },
		{ kind: 'report', value: run['report'] },
		{ kind: 'inspection_report', value: inspection['report'] },
	] as const;
	const beforeRequests = backend.captures.length;
	const readbacks = await Promise.all(
		roots.map(async (rootValue) => {
			const saved = recordSchema.parse(serialize(rootValue));
			requireSdk(
				saved['kind'] === 'serialized' && typeof saved['json'] === 'string',
				`${rootValue.kind}: writer refused actual SDK value`,
			);
			const decoded = recordSchema.parse(
				await boundedSdkWait(deserialize(saved['json']), `decode ${rootValue.kind}`),
			);
			saveSdkEvidence(path.join(evidence, `codec-root-${rootValue.kind}.json`), { saved, decoded });
			return decoded;
		}),
	);
	for (const [index, rootValue] of roots.entries()) {
		const decoded = readbacks[index];
		requireSdk(
			decoded && decoded['kind'] === 'decoded',
			`${rootValue.kind}: reader refused actual SDK value`,
		);
		requireSdk(
			isDeepStrictEqual(decoded['value'], rootValue),
			`${rootValue.kind}: decoded value differs from original SDK value`,
		);
		requireDeepFrozenSdkValue(decoded['value']);
	}
	requireSdk(
		backend.captures.length === beforeRequests,
		'Codec root roundtrips dispatched model requests',
	);
	return {
		observed: [
			'all eight SDK document roots roundtrip actual installed values with frozen exact fields',
			'no model request during codec root roundtrips',
		],
		gaps: [
			'independently authored structured JSON boundaries and selected-root widening counterfactuals',
			'codec credential/file/engine purity beyond zero controlled HTTP',
		],
	};
}
