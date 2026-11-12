import type { SdkFailure } from './contracts/contracts.ts';
// load, check and run over a resolved native binding, with closed endings.
//
// Before-dispatch failures have no effects. Once the native call starts, an
// unreadable reply or a rejected native promise is a failed outcome whose effects
// are unknown; the SDK never retries a call.
import type { BindingResolution, NativeBinding, NativeCallCancellation } from './native-binding.ts';
import type {
	CallOptions,
	CheckOutcome,
	ConfigurationInput,
	ContractInfo,
	LoadOptions,
	LoadOutcome,
	RunOptions,
	RunOutcome,
} from './public-types.ts';
import { deepFreeze } from './readonly-values.ts';
import {
	isCheckOutcome,
	isLoadOutcome,
	isRunOutcome,
	parseExactJson,
	widenNamed,
} from './reply-decoding.ts';
import { afterDispatch, beforeDispatch } from './sdk-failures.ts';
import { encodeForNative } from './wire-encoding.ts';

export interface SdkCalls {
	contractInfo(): Promise<
		| { readonly kind: 'ready'; readonly contract: ContractInfo }
		| { readonly kind: 'failed'; readonly failure: SdkFailure }
	>;
	load(options: LoadOptions): Promise<LoadOutcome>;
	check(input: ConfigurationInput, options?: CallOptions): Promise<CheckOutcome>;
	run(input: ConfigurationInput, options: RunOptions): Promise<RunOutcome>;
}

type DecodedReply<TOutcome> =
	| { readonly kind: 'decoded'; readonly outcome: TOutcome }
	| { readonly kind: 'invalid'; readonly message: string };

function decodeReply<TOutcome>(
	text: string,
	root: string,
	accepts: (value: unknown) => value is TOutcome,
): DecodedReply<TOutcome> {
	let parsed: unknown;
	try {
		parsed = parseExactJson(text);
	} catch (error) {
		return {
			kind: 'invalid',
			message: `native reply is not JSON: ${error instanceof Error ? error.message : String(error)}`,
		};
	}
	const widened = widenNamed(parsed, root);
	return accepts(widened)
		? { kind: 'decoded', outcome: widened }
		: { kind: 'invalid', message: `native reply is not a ${root}` };
}

/** The executable configuration of either input form; a receipt never executes. */
function executableConfiguration(input: ConfigurationInput): unknown {
	return 'configuration' in input ? input.configuration : input;
}

function describeRejection(error: unknown): string {
	return error instanceof Error ? error.message : String(error);
}

/** Tie an AbortSignal to one native cancellation for the duration of one call. */
async function withCancellation<TValue>(
	binding: NativeBinding,
	signal: AbortSignal | undefined,
	call: (cancellation: NativeCallCancellation) => Promise<TValue>,
): Promise<TValue> {
	const cancellation = new binding.CallCancellation();
	const abort = (): void => cancellation.cancel();
	signal?.addEventListener('abort', abort, { once: true });
	if (signal?.aborted === true) cancellation.cancel();
	try {
		return await call(cancellation);
	} finally {
		signal?.removeEventListener('abort', abort);
	}
}

/** Build the SDK calls over a binding resolver; the addon loads lazily on first use. */
export function createSdkCalls(resolveBinding: () => BindingResolution): SdkCalls {
	let resolution: BindingResolution | undefined;
	const binding = (): BindingResolution => {
		resolution ??= resolveBinding();
		return resolution;
	};

	return {
		async contractInfo() {
			const resolved = binding();
			return resolved.kind === 'ready'
				? { kind: 'ready', contract: resolved.contract }
				: { kind: 'failed', failure: resolved.failure };
		},

		async load(options: LoadOptions): Promise<LoadOutcome> {
			const resolved = binding();
			if (resolved.kind === 'failed') return { kind: 'failed', failure: resolved.failure };
			const encoded = encodeForNative(options);
			if (encoded.kind === 'rejected') {
				return { kind: 'failed', failure: beforeDispatch('encoding_rejected', encoded.message) };
			}
			let reply: string;
			try {
				reply = await resolved.binding.load(encoded.json);
			} catch (error) {
				return {
					kind: 'failed',
					failure: afterDispatch('native_call_failed', describeRejection(error)),
				};
			}
			const decoded = decodeReply(reply, 'LoadOutcome', isLoadOutcome);
			if (decoded.kind === 'invalid') {
				return { kind: 'failed', failure: afterDispatch('native_reply_invalid', decoded.message) };
			}
			return deepFreeze(decoded.outcome);
		},

		async check(input: ConfigurationInput, options?: CallOptions): Promise<CheckOutcome> {
			// An already-aborted signal wins over every other ending and starts no work.
			if (options?.signal?.aborted === true) return { kind: 'cancelled', diagnostics: [] };
			const resolved = binding();
			if (resolved.kind === 'failed') return { kind: 'failed', failure: resolved.failure };
			const encoded = encodeForNative(executableConfiguration(input));
			if (encoded.kind === 'rejected') {
				return { kind: 'failed', failure: beforeDispatch('encoding_rejected', encoded.message) };
			}
			let reply: string;
			try {
				reply = await withCancellation(resolved.binding, options?.signal, (cancellation) =>
					resolved.binding.check(encoded.json, cancellation),
				);
			} catch (error) {
				return {
					kind: 'failed',
					failure: afterDispatch('native_call_failed', describeRejection(error)),
				};
			}
			const decoded = decodeReply(reply, 'CheckOutcome', isCheckOutcome);
			if (decoded.kind === 'invalid') {
				return { kind: 'failed', failure: afterDispatch('native_reply_invalid', decoded.message) };
			}
			return deepFreeze(decoded.outcome);
		},

		async run(input: ConfigurationInput, options: RunOptions): Promise<RunOutcome> {
			if (options.signal?.aborted === true) return { kind: 'cancelled', report: null };
			const resolved = binding();
			if (resolved.kind === 'failed')
				return { kind: 'failed', failure: resolved.failure, report: null };
			const configuration = encodeForNative(executableConfiguration(input));
			const request = encodeForNative({
				root: options.root,
				scope: options.scope,
				...(options.paths === undefined ? {} : { paths: options.paths }),
			});
			if (configuration.kind === 'rejected' || request.kind === 'rejected') {
				const message =
					configuration.kind === 'rejected'
						? configuration.message
						: request.kind === 'rejected'
							? request.message
							: '';
				return {
					kind: 'failed',
					failure: beforeDispatch('encoding_rejected', message),
					report: null,
				};
			}
			let reply: string;
			try {
				reply = await withCancellation(resolved.binding, options.signal, (cancellation) =>
					resolved.binding.run(configuration.json, request.json, cancellation),
				);
			} catch (error) {
				return {
					kind: 'failed',
					failure: afterDispatch('native_call_failed', describeRejection(error)),
					report: null,
				};
			}
			const decoded = decodeReply(reply, 'RunOutcome', isRunOutcome);
			if (decoded.kind === 'invalid') {
				return {
					kind: 'failed',
					failure: afterDispatch('native_reply_invalid', decoded.message),
					report: null,
				};
			}
			return deepFreeze(decoded.outcome);
		},
	};
}
