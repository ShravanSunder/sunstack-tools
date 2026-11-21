import type { AdapterRegistry } from './adapter-registry.ts';
import { encodeAdapterMappings } from './adapter-registry.ts';
import type * as Contracts from './contracts/contracts.ts';
import { installedBindingSources, resolveBinding } from './native-binding.ts';
import type { BindingResolution, NativeCallCancellation } from './native-binding.ts';
import type { CallOptions, SdkFailure } from './public-types.ts';
import type { DeepReadonly } from './readonly-values.ts';
import { deepFreeze } from './readonly-values.ts';
import { parseExactJson, widenNamed } from './reply-decoding.ts';
import { describeRejection, selectSignal, withCancellation } from './sdk-calls.ts';
import { afterDispatch, beforeDispatch } from './sdk-failures.ts';
import { encodeForNative } from './wire-encoding.ts';

export { deserialize, serialize } from './index.ts';
export { AdapterRegistry, conversation } from './adapter-registry.ts';
export type {
	ConversationSource,
	TranscriptAdapterMapping,
	TranscriptRecordMapping,
} from './adapter-registry.ts';
export { builtinTranscriptFormats } from './contracts/contracts.ts';
export type { BuiltinTranscriptFormat } from './contracts/contracts.ts';

export type BindingGroup = DeepReadonly<Contracts.BindingGroup>;
export type InputBinding = DeepReadonly<Contracts.InputBinding>;
export type InvocationOrigin = DeepReadonly<Contracts.InvocationOrigin>;
export type RuleRunProfile = DeepReadonly<Contracts.RuleRunProfile>;
export type InputContract = DeepReadonly<Contracts.RuleSetLoadRequest['input_contract']>;
export type RuleInvocationInput = DeepReadonly<Contracts.RuleInvocationInput>;
export type RuleInvocationOutcome = DeepReadonly<Contracts.RuleInvocationOutcome>;
export type RulePreviewOutcome = DeepReadonly<Contracts.RulePreviewOutcome>;
export type EvidencePlanInput = DeepReadonly<Contracts.EvidencePlanInput>;
export type EvidencePlanOutcome = DeepReadonly<Contracts.EvidencePlanOutcome>;
export type GeneralRuleOutcome<TOutcome> =
	| TOutcome
	| { readonly kind: 'failed'; readonly failure: SdkFailure };

/** Evidence acquisition options for inspect, preview and evaluation. */
export interface RuleCallOptions extends CallOptions {
	readonly adapterRegistry?: AdapterRegistry<string>;
}

let binding: BindingResolution | undefined;

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isEvaluationOutcome(value: unknown): value is Contracts.RuleInvocationOutcome {
	if (!isRecord(value)) return false;
	switch (value['kind']) {
		case 'complete_report':
		case 'incomplete_report':
			return isRecord(value['report']);
		case 'invalid':
			return isRecord(value['diagnostics']);
		case 'preparation_failed':
			return isRecord(value['diagnostics']) && isRecord(value['measurements']);
		case 'cancelled':
			return (
				isRecord(value['progress']) &&
				isRecord(value['diagnostics']) &&
				isRecord(value['measurements'])
			);
		default:
			return false;
	}
}

function isEvidenceOutcome(value: unknown): value is Contracts.EvidencePlanOutcome {
	if (!isRecord(value)) return false;
	switch (value['kind']) {
		case 'prepared':
			return Array.isArray(value['selections']) && isRecord(value['measurements']);
		case 'incomplete':
			return (
				Array.isArray(value['selections']) &&
				isRecord(value['diagnostics']) &&
				isRecord(value['measurements'])
			);
		case 'refused':
			return isRecord(value['diagnostics']);
		case 'cancelled':
			return (
				isRecord(value['progress']) &&
				isRecord(value['diagnostics']) &&
				isRecord(value['measurements'])
			);
		default:
			return false;
	}
}

function isPreviewOutcome(value: unknown): value is Contracts.RulePreviewOutcome {
	if (!isRecord(value)) return false;
	switch (value['kind']) {
		case 'ready_single':
			return (
				isRecord(value['request']) &&
				typeof value['request']['body'] === 'string' &&
				isRecord(value['preparation'])
			);
		case 'no_model_work':
			return isRecord(value['report']);
		case 'pending_dependency':
		case 'unsupported_multiple':
			return Array.isArray(value['ready']) && isRecord(value['preparation']);
		case 'unavailable':
			return isRecord(value['outcome']);
		default:
			return false;
	}
}

function isReply<TOutcome>(
	value: unknown,
	accepts: (outcome: unknown) => outcome is TOutcome,
): value is Contracts.GeneralRuleCall<TOutcome> {
	return (
		isRecord(value) &&
		((value['kind'] === 'received' && accepts(value['outcome'])) ||
			(value['kind'] === 'failed' && isRecord(value['failure'])))
	);
}

type NativeRuleMethod = (
	input: string,
	cancellation: NativeCallCancellation,
	ruleSetOrMappings?: object | string,
	adapterMappings?: string,
) => Promise<string>;

function isNativeRuleMethod(value: unknown): value is NativeRuleMethod {
	return typeof value === 'function';
}

interface GeneralRuleOperation<TOutcome> {
	readonly method: 'evaluateRules' | 'inspectEvidence' | 'prepareRules';
	readonly root: 'RuleInvocationOutcome' | 'EvidencePlanOutcome' | 'RulePreviewOutcome';
	readonly accepts: (value: unknown) => value is TOutcome;
}

interface GeneralRuleCallOptions {
	readonly input: unknown;
	readonly options: RuleCallOptions | undefined;
	readonly ruleSet?: object;
}

async function callRules<TOutcome>(
	operation: GeneralRuleOperation<TOutcome>,
	{ input, options, ruleSet }: GeneralRuleCallOptions,
): Promise<GeneralRuleOutcome<DeepReadonly<TOutcome>>> {
	binding ??= resolveBinding(installedBindingSources());
	if (binding.kind === 'failed') return { kind: 'failed', failure: binding.failure };
	const addon = binding.binding;
	const method: unknown = Reflect.get(addon, operation.method);
	if (!isNativeRuleMethod(method)) {
		return {
			kind: 'failed',
			failure: beforeDispatch(
				'contract_mismatch',
				'Install a Sunstack addon with general rules support.',
			),
		};
	}
	const signal = selectSignal(options);
	if (signal.kind === 'rejected') {
		return { kind: 'failed', failure: beforeDispatch('encoding_rejected', signal.message) };
	}
	const encoded = encodeForNative(input);
	if (encoded.kind === 'rejected') {
		return { kind: 'failed', failure: beforeDispatch('encoding_rejected', encoded.message) };
	}
	let mappings: string | undefined;
	try {
		const registry = options?.adapterRegistry;
		if (registry !== undefined) mappings = encodeAdapterMappings(registry);
	} catch (error) {
		return {
			kind: 'failed',
			failure: beforeDispatch('encoding_rejected', describeRejection(error)),
		};
	}
	if (
		mappings !== undefined &&
		typeof Reflect.get(addon, 'supportsTranscriptAdapterMappings') !== 'function'
	) {
		return {
			kind: 'failed',
			failure: beforeDispatch(
				'contract_mismatch',
				'Install a Sunstack addon with transcript adapter support.',
			),
		};
	}
	let reply: string;
	try {
		reply = await withCancellation(addon, signal.signal, (cancellation) =>
			operation.method === 'inspectEvidence'
				? method.call(addon, encoded.json, cancellation, mappings)
				: method.call(addon, encoded.json, cancellation, ruleSet, mappings),
		);
	} catch (error) {
		return {
			kind: 'failed',
			failure: afterDispatch('native_call_failed', describeRejection(error)),
		};
	}
	let parsed: unknown;
	try {
		parsed = parseExactJson(reply);
	} catch {
		parsed = undefined;
	}
	if (!isReply(parsed, operation.accepts)) {
		return {
			kind: 'failed',
			failure: afterDispatch(
				'native_reply_invalid',
				`Native ${operation.method} returned an invalid outcome envelope.`,
			),
		};
	}
	if (parsed.kind === 'failed') return deepFreeze(parsed);
	widenNamed(parsed.outcome, operation.root);
	return deepFreeze(parsed.outcome);
}

const ruleSetBrand: unique symbol = Symbol('admitted rule set');
const nativeRuleSets = new WeakMap<object, object>();

/** This SDK instance's admitted definition and explicit native run profile. */
export interface AdmittedRuleSet<TContract extends InputContract> {
	readonly [ruleSetBrand]: TContract;
}

export interface LoadRuleSetOptions<TContract extends InputContract> {
	readonly path: string;
	readonly inputContract: TContract;
	readonly profile: RuleRunProfile;
}

export type RuleSetLoadOutcome<TContract extends InputContract> =
	| { readonly kind: 'ready'; readonly ruleSet: AdmittedRuleSet<TContract> }
	| Exclude<DeepReadonly<Contracts.RuleSetLoadStatus>, { readonly kind: 'ready' }>;

type NamedBindings<TContract extends InputContract> = {
	readonly [TName in keyof TContract]: Extract<
		InputBinding,
		{ kind: NonNullable<TContract[TName]>['kind'] }
	>;
};

export interface BoundRuleInvocation<TContract extends InputContract> {
	readonly inputs: NamedBindings<TContract>;
	readonly origin: InvocationOrigin;
}

type LoadRuleSetMethod = (input: string, cancellation: NativeCallCancellation) => Promise<unknown>;

function isLoadRuleSetMethod(value: unknown): value is LoadRuleSetMethod {
	return typeof value === 'function';
}

function isRuleSetLoadStatus(value: unknown): value is Contracts.RuleSetLoadStatus {
	if (!isRecord(value)) return false;
	switch (value['kind']) {
		case 'ready':
		case 'cancelled':
			return true;
		case 'invalid':
			return isRecord(value['diagnostics']);
		case 'failed':
			return isRecord(value['failure']);
		default:
			return false;
	}
}

/** Check the input companion and retain a native snapshot of the definition and profile. */
export async function loadRuleSet<const TContract extends InputContract>(
	request: LoadRuleSetOptions<TContract>,
	options?: CallOptions,
): Promise<RuleSetLoadOutcome<TContract>> {
	binding ??= resolveBinding(installedBindingSources());
	if (binding.kind === 'failed') return { kind: 'failed', failure: binding.failure };
	const addon = binding.binding;
	const method: unknown = Reflect.get(addon, 'loadRuleSet');
	if (!isLoadRuleSetMethod(method))
		return {
			kind: 'failed',
			failure: beforeDispatch('contract_mismatch', 'Install a matching Sunstack addon.'),
		};

	const signal = selectSignal(options);
	if (signal.kind === 'rejected')
		return { kind: 'failed', failure: beforeDispatch('encoding_rejected', signal.message) };

	let inputContract: TContract;
	let reply: unknown;
	try {
		inputContract = request.inputContract;
		const wire = { path: request.path, input_contract: inputContract, profile: request.profile };
		const encoded = encodeForNative(wire);
		if (encoded.kind === 'rejected')
			return { kind: 'failed', failure: beforeDispatch('encoding_rejected', encoded.message) };
		reply = await withCancellation(addon, signal.signal, (cancellation) =>
			method.call(addon, encoded.json, cancellation),
		);
	} catch (error) {
		return {
			kind: 'failed',
			failure: afterDispatch('native_call_failed', describeRejection(error)),
		};
	}

	let status: unknown;
	if (isRecord(reply) && typeof reply['json'] === 'string') {
		try {
			status = widenNamed(parseExactJson(reply['json']), 'RuleSetLoadStatus');
		} catch {
			status = undefined;
		}
		if (isRuleSetLoadStatus(status)) {
			if (status.kind !== 'ready') return deepFreeze(status);
			const nativeRuleSet = reply['ruleSet'];
			if (typeof nativeRuleSet === 'object' && nativeRuleSet !== null) {
				const ruleSet: AdmittedRuleSet<TContract> = Object.freeze({
					[ruleSetBrand]: inputContract,
				});
				nativeRuleSets.set(ruleSet, nativeRuleSet);
				return { kind: 'ready', ruleSet };
			}
		}
	}
	return {
		kind: 'failed',
		failure: afterDispatch(
			'native_reply_invalid',
			'Native admission returned an invalid outcome or handle.',
		),
	};
}

type DocumentBinding = Extract<InputBinding, { kind: 'document' }>;

export function document(input: Omit<DocumentBinding, 'kind'>): DocumentBinding {
	return { kind: 'document', ...input };
}

function selectRuleInput(
	definition: RuleInvocationInput | AdmittedRuleSet<InputContract>,
	invocationOrOptions: BoundRuleInvocation<InputContract> | RuleCallOptions | undefined,
	options: RuleCallOptions | undefined,
): GeneralRuleCallOptions | { readonly kind: 'failed'; readonly failure: SdkFailure } {
	if (!(ruleSetBrand in definition)) {
		const rawOptions =
			invocationOrOptions !== undefined && 'inputs' in invocationOrOptions
				? undefined
				: invocationOrOptions;
		return { input: definition, options: rawOptions };
	}
	const ruleSet = nativeRuleSets.get(definition);
	if (ruleSet === undefined) {
		return {
			kind: 'failed',
			failure: beforeDispatch(
				'encoding_rejected',
				'Supply a rule set loaded by this SDK instance.',
			),
		};
	}
	return { input: invocationOrOptions, options, ruleSet };
}

/** Evaluate file-based native rules over this invocation's bound input sources. */
export function evaluateRules(
	input: RuleInvocationInput,
	options?: RuleCallOptions,
): Promise<GeneralRuleOutcome<RuleInvocationOutcome>>;
export function evaluateRules<const TContract extends InputContract>(
	ruleSet: AdmittedRuleSet<TContract>,
	invocation: BoundRuleInvocation<NoInfer<TContract>>,
	options?: RuleCallOptions,
): Promise<GeneralRuleOutcome<RuleInvocationOutcome>>;
export async function evaluateRules(
	definition: RuleInvocationInput | AdmittedRuleSet<InputContract>,
	invocationOrOptions?: BoundRuleInvocation<InputContract> | RuleCallOptions,
	options?: RuleCallOptions,
): Promise<GeneralRuleOutcome<RuleInvocationOutcome>> {
	const selected = selectRuleInput(definition, invocationOrOptions, options);
	if ('kind' in selected) return selected;
	return callRules(
		{ method: 'evaluateRules', root: 'RuleInvocationOutcome', accepts: isEvaluationOutcome },
		selected,
	);
}

/** Read selected evidence with coverage and original locations without a model. */
export function inspectEvidence(
	input: EvidencePlanInput,
	options?: RuleCallOptions,
): Promise<GeneralRuleOutcome<EvidencePlanOutcome>> {
	return callRules(
		{ method: 'inspectEvidence', root: 'EvidencePlanOutcome', accepts: isEvidenceOutcome },
		{ input, options },
	);
}

/** Inspect exact model request bodies and which work is ready without credentials or dispatch. */
export function prepareRules(
	input: RuleInvocationInput,
	options?: RuleCallOptions,
): Promise<GeneralRuleOutcome<RulePreviewOutcome>>;
export function prepareRules<const TContract extends InputContract>(
	ruleSet: AdmittedRuleSet<TContract>,
	invocation: BoundRuleInvocation<NoInfer<TContract>>,
	options?: RuleCallOptions,
): Promise<GeneralRuleOutcome<RulePreviewOutcome>>;
export async function prepareRules(
	definition: RuleInvocationInput | AdmittedRuleSet<InputContract>,
	invocationOrOptions?: BoundRuleInvocation<InputContract> | RuleCallOptions,
	options?: RuleCallOptions,
): Promise<GeneralRuleOutcome<RulePreviewOutcome>> {
	const selected = selectRuleInput(definition, invocationOrOptions, options);
	if ('kind' in selected) return selected;
	return callRules(
		{ method: 'prepareRules', root: 'RulePreviewOutcome', accepts: isPreviewOutcome },
		selected,
	);
}
