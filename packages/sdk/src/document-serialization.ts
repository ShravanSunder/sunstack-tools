// The caller owns storage. This writer owns exact numeric tokens and SDK framing.
import { CONTRACT_REVISION } from './contracts/contract-revision.ts';
import type { SdkDocument, SdkDocumentValue, SerializationOutcome } from './public-types.ts';
import { encodeForNative } from './wire-encoding.ts';

/** Save a known SDK value; encoding success does not admit or execute it. */
export function serialize(value: SdkDocumentValue): SerializationOutcome {
	if (!('rawJSON' in JSON) || typeof JSON.rawJSON !== 'function') {
		return {
			kind: 'rejected',
			reason: 'unsupported_runtime',
			message: 'This runtime lacks JSON.rawJSON for exact integers.',
		};
	}
	const document = { contract_revision: CONTRACT_REVISION, payload: value } satisfies SdkDocument;
	const encoded = encodeForNative(document);
	return encoded.kind === 'encoded'
		? { kind: 'serialized', json: encoded.json }
		: { kind: 'rejected', reason: 'invalid_data', message: encoded.message };
}
