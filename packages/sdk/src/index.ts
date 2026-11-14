// @sunstack/sdk: load, check and run Sunstack configurations from TypeScript.
//
// Declarations come from the Rust contracts; the Rust engine decides validity,
// executes rules and returns typed outcomes. No call throws for a domain or
// detectable native ending.
import { installedBindingSources, resolveBinding } from './native-binding.ts';
import type {
	DeserializationOutcome,
	InspectionOutcome,
	InspectOptions,
	CallOptions,
	CheckOutcome,
	ConfigurationInput,
	ContractInfo,
	LoadOptions,
	LoadOutcome,
	RunOptions,
	RunOutcome,
	SdkFailure,
} from './public-types.ts';
import { createSdkCalls } from './sdk-calls.ts';

export type * from './public-types.ts';

const calls = createSdkCalls(() => resolveBinding(installedBindingSources()));

/** Discover and load an existing configuration, capturing every definition it references. */
export function load(options: LoadOptions): Promise<LoadOutcome> {
	return calls.load(options);
}

/** Admit a configuration without inference: no provider, credential or checked file is touched. */
export function check(input: ConfigurationInput, options?: CallOptions): Promise<CheckOutcome> {
	return calls.check(input, options);
}

/** Run a configuration's rules against an explicit absolute checked root. */
export function run(input: ConfigurationInput, options: RunOptions): Promise<RunOutcome> {
	return calls.run(input, options);
}

/** The installed addon's verified release, contract revision and target. */
export function contractInfo(): Promise<
	| { readonly kind: 'ready'; readonly contract: ContractInfo }
	| { readonly kind: 'failed'; readonly failure: SdkFailure }
> {
	return calls.contractInfo();
}

/** Acquire and select evidence without evaluating rule questions. */
export function inspect(
	input: ConfigurationInput,
	options: InspectOptions,
): Promise<InspectionOutcome> {
	return calls.inspect(input, options);
}

/** Restore a current-revision SDK document through the matched Rust codec. */
export function deserialize(json: string): Promise<DeserializationOutcome> {
	return calls.deserialize(json);
}

export { serialize } from './document-serialization.ts';
