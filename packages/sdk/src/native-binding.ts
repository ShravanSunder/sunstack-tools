// Locate, load and verify the platform addon before any engine call.
import { createRequire } from 'node:module';

import { CONTRACT_REVISION } from './contracts/contract-revision.ts';
import type { ContractInfo } from './contracts/contracts.ts';
import type { BeforeDispatchFailure } from './sdk-failures.ts';
import { beforeDispatch } from './sdk-failures.ts';

/** One call's native cancellation; cancelling it never affects another call. */
export interface NativeCallCancellation {
	cancel(): void;
}

/** The functions the Node-API addon exports. Replies are contract JSON text. */
export interface NativeBinding {
	contractInfo(): string;
	CallCancellation: new () => NativeCallCancellation;
	deserialize(requestJson: string): Promise<string>;
	inspect(
		configurationJson: string,
		requestJson: string,
		cancellation: NativeCallCancellation,
	): Promise<string>;
	load(optionsJson: string): Promise<string>;
	check(configurationJson: string, cancellation: NativeCallCancellation): Promise<string>;
	run(
		configurationJson: string,
		requestJson: string,
		cancellation: NativeCallCancellation,
	): Promise<string>;
}

export type BindingResolution =
	| { readonly kind: 'ready'; readonly binding: NativeBinding; readonly contract: ContractInfo }
	| { readonly kind: 'failed'; readonly failure: BeforeDispatchFailure };

/** A supported install target and the Rust target its addon must report. */
export interface PlatformCell {
	readonly key: string;
	readonly platform: NodeJS.Platform;
	readonly arch: string;
	readonly libc: 'glibc' | 'musl' | null;
	readonly target: string;
}

export const platformCells: readonly PlatformCell[] = [
	{
		key: 'darwin-arm64',
		platform: 'darwin',
		arch: 'arm64',
		libc: null,
		target: 'aarch64-apple-darwin',
	},
	{ key: 'darwin-x64', platform: 'darwin', arch: 'x64', libc: null, target: 'x86_64-apple-darwin' },
	{
		key: 'linux-arm64-gnu',
		platform: 'linux',
		arch: 'arm64',
		libc: 'glibc',
		target: 'aarch64-unknown-linux-gnu',
	},
	{
		key: 'linux-arm64-musl',
		platform: 'linux',
		arch: 'arm64',
		libc: 'musl',
		target: 'aarch64-unknown-linux-musl',
	},
	{
		key: 'linux-x64-gnu',
		platform: 'linux',
		arch: 'x64',
		libc: 'glibc',
		target: 'x86_64-unknown-linux-gnu',
	},
	{
		key: 'linux-x64-musl',
		platform: 'linux',
		arch: 'x64',
		libc: 'musl',
		target: 'x86_64-unknown-linux-musl',
	},
];

/** The host facts that decide which addon is loadable. */
export interface HostRuntime {
	readonly nodeVersion: string;
	readonly platform: NodeJS.Platform;
	readonly arch: string;
	readonly libc: 'glibc' | 'musl' | null;
	readonly hasRawJson: boolean;
}

function hasGlibcRuntime(report: unknown): boolean {
	if (typeof report !== 'object' || report === null || !('header' in report)) return false;
	const header = report.header;
	return (
		typeof header === 'object' &&
		header !== null &&
		'glibcVersionRuntime' in header &&
		typeof header.glibcVersionRuntime === 'string'
	);
}

export function currentHostRuntime(): HostRuntime {
	const libc =
		process.platform === 'linux'
			? hasGlibcRuntime(process.report.getReport())
				? 'glibc'
				: 'musl'
			: null;
	return {
		nodeVersion: process.versions.node,
		platform: process.platform,
		arch: process.arch,
		libc,
		hasRawJson: 'rawJSON' in JSON && typeof JSON.rawJSON === 'function',
	};
}

/** Exactly one supported cell matches the host, or none does. */
export function selectPlatformCell(host: HostRuntime): PlatformCell | null {
	return (
		platformCells.find(
			(cell) =>
				cell.platform === host.platform && cell.arch === host.arch && cell.libc === host.libc,
		) ?? null
	);
}

const minimumNodeMajor = 26;

function isNativeBinding(value: unknown): value is NativeBinding {
	if (typeof value !== 'object' || value === null) return false;
	return (
		'contractInfo' in value &&
		typeof value.contractInfo === 'function' &&
		'CallCancellation' in value &&
		typeof value.CallCancellation === 'function' &&
		'load' in value &&
		typeof value.load === 'function' &&
		'check' in value &&
		typeof value.check === 'function' &&
		'inspect' in value &&
		typeof value.inspect === 'function' &&
		'deserialize' in value &&
		typeof value.deserialize === 'function' &&
		'run' in value &&
		typeof value.run === 'function'
	);
}

function isContractInfo(value: unknown): value is ContractInfo {
	return (
		typeof value === 'object' &&
		value !== null &&
		'sdk_release' in value &&
		typeof value.sdk_release === 'string' &&
		'contract_revision' in value &&
		typeof value.contract_revision === 'number' &&
		'target' in value &&
		typeof value.target === 'string'
	);
}

function errorCode(error: unknown): string | null {
	return typeof error === 'object' &&
		error !== null &&
		'code' in error &&
		typeof error.code === 'string'
		? error.code
		: null;
}

/** The inputs to one binding resolution; tests substitute the module loader. */
export interface BindingSources {
	readonly host: HostRuntime;
	readonly sdkRelease: string;
	readonly loadAddon: (cell: PlatformCell) => unknown;
}

/**
 * Classify the host, load its addon and compare the exact release, contract
 * revision and target. Every refusal happens before any engine call.
 */
export function resolveBinding(sources: BindingSources): BindingResolution {
	const { host } = sources;
	const major = Number.parseInt(host.nodeVersion.split('.')[0] ?? '', 10);
	if (!Number.isInteger(major) || major < minimumNodeMajor || !host.hasRawJson) {
		return {
			kind: 'failed',
			failure: beforeDispatch(
				'unsupported_runtime',
				`Node ${host.nodeVersion} is unsupported; the SDK requires Node ${minimumNodeMajor} or later.`,
			),
		};
	}
	const cell = selectPlatformCell(host);
	if (cell === null) {
		return {
			kind: 'failed',
			failure: beforeDispatch(
				'unsupported_platform',
				`No Sunstack SDK addon is built for ${host.platform}/${host.arch}${host.libc === null ? '' : `/${host.libc}`}.`,
			),
		};
	}
	let addon: unknown;
	try {
		addon = sources.loadAddon(cell);
	} catch (error) {
		const code = errorCode(error);
		const reason = error instanceof Error ? error.message : String(error);
		return {
			kind: 'failed',
			failure: beforeDispatch(
				'addon_missing',
				code === 'MODULE_NOT_FOUND' || code === 'ERR_MODULE_NOT_FOUND'
					? `The addon package @sunstack/sdk-${cell.key} is not installed. Reinstall @sunstack/sdk ${sources.sdkRelease} for this platform.`
					: `The addon for ${cell.key} could not be loaded: ${reason}`,
			),
		};
	}
	if (!isNativeBinding(addon)) {
		return {
			kind: 'failed',
			failure: beforeDispatch(
				'contract_mismatch',
				`The ${cell.key} addon does not export the SDK contract.`,
			),
		};
	}
	let contract: unknown;
	try {
		contract = JSON.parse(addon.contractInfo());
	} catch {
		contract = null;
	}
	if (
		!isContractInfo(contract) ||
		contract.sdk_release !== sources.sdkRelease ||
		contract.contract_revision !== CONTRACT_REVISION ||
		contract.target !== cell.target
	) {
		const found = isContractInfo(contract)
			? `release ${contract.sdk_release}, contract ${contract.contract_revision}, target ${contract.target}`
			: 'no readable contract';
		return {
			kind: 'failed',
			failure: beforeDispatch(
				'contract_mismatch',
				`The installed addon (${found}) does not match @sunstack/sdk ${sources.sdkRelease} (contract ${CONTRACT_REVISION}, target ${cell.target}). Reinstall matching packages.`,
			),
		};
	}
	return { kind: 'ready', binding: addon, contract };
}

const requireFromHere = createRequire(import.meta.url);

function packageRelease(): string {
	const manifest: unknown = requireFromHere('../package.json');
	return typeof manifest === 'object' &&
		manifest !== null &&
		'version' in manifest &&
		typeof manifest.version === 'string'
		? manifest.version
		: '';
}

/** Load this host's addon package from the installed dependency tree. */
export function installedBindingSources(): BindingSources {
	return {
		host: currentHostRuntime(),
		sdkRelease: packageRelease(),
		loadAddon: (cell): unknown => requireFromHere(`@sunstack/sdk-${cell.key}`),
	};
}
