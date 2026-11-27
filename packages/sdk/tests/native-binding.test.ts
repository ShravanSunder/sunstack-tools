import assert from 'node:assert/strict';
import { test } from 'node:test';

import { CONTRACT_REVISION } from '../src/contracts/contract-revision.ts';
import type { BindingResolution, HostRuntime } from '../src/native-binding.ts';
import {
	installedBindingSources,
	platformCells,
	resolveBinding,
	selectPlatformCell,
} from '../src/native-binding.ts';

const supportedHost: HostRuntime = {
	nodeVersion: '26.11.1',
	platform: 'linux',
	arch: 'x64',
	libc: 'glibc',
	hasRawJson: true,
};

function failureReason(resolution: BindingResolution): string {
	assert.equal(resolution.kind, 'failed');
	return resolution.kind === 'failed' ? resolution.failure.reason : '';
}

function addonReporting(contract: unknown): unknown {
	return {
		contractInfo: (): string => JSON.stringify(contract),
		CallCancellation: class {
			cancel(): void {}
		},
		load: (): never => assert.fail('load must not run'),
		deserialize: (): never => assert.fail('deserialize must not run'),
		inspect: (): never => assert.fail('inspect must not run'),
		check: (): never => assert.fail('check must not run'),
		run: (): never => assert.fail('run must not run'),
	};
}

await test('the three supported hosts select their exact native targets', () => {
	assert.deepEqual(
		platformCells.map(({ key, target }) => [key, target]),
		[
			['darwin-arm64', 'aarch64-apple-darwin'],
			['linux-arm64-gnu', 'aarch64-unknown-linux-gnu'],
			['linux-x64-gnu', 'x86_64-unknown-linux-gnu'],
		],
	);
	for (const cell of platformCells) {
		assert.equal(selectPlatformCell({ ...supportedHost, ...cell }), cell);
	}
});

const neverLoad = (): never => assert.fail('no addon load');

await test('unsupported runtimes and platforms fail before any addon is loaded', () => {
	assert.equal(
		failureReason(
			resolveBinding({
				host: { ...supportedHost, nodeVersion: '24.9.0' },
				sdkRelease: '0.12.0',
				loadAddon: neverLoad,
			}),
		),
		'unsupported_runtime',
	);
	assert.equal(
		failureReason(
			resolveBinding({
				host: { ...supportedHost, hasRawJson: false },
				sdkRelease: '0.12.0',
				loadAddon: neverLoad,
			}),
		),
		'unsupported_runtime',
	);
	for (const host of [
		{ ...supportedHost, platform: 'win32', libc: null },
		{ ...supportedHost, platform: 'darwin', arch: 'x64', libc: null },
		{ ...supportedHost, arch: 'arm64', libc: 'musl' },
		{ ...supportedHost, arch: 'x64', libc: 'musl' },
	] as const) {
		assert.equal(
			failureReason(resolveBinding({ host, sdkRelease: '0.12.0', loadAddon: neverLoad })),
			'unsupported_platform',
		);
	}
});

await test('a missing addon package is a closed failure naming the package', () => {
	const resolution = resolveBinding({
		host: supportedHost,
		sdkRelease: '0.12.0',
		loadAddon: () => {
			throw Object.assign(new Error('Cannot find module'), { code: 'MODULE_NOT_FOUND' });
		},
	});
	assert.equal(failureReason(resolution), 'addon_missing');
	assert.match(
		resolution.kind === 'failed' ? resolution.failure.message : '',
		/@sunstack\/sdk-linux-x64-gnu/u,
	);
});

await test('release, revision and target skew are refused before any engine call', () => {
	const exact = {
		sdk_release: '0.12.0',
		contract_revision: CONTRACT_REVISION,
		target: 'x86_64-unknown-linux-gnu',
	};
	for (const skewed of [
		{ ...exact, sdk_release: '0.11.9' },
		{ ...exact, contract_revision: CONTRACT_REVISION + 1 },
		{ ...exact, target: 'x86_64-unknown-linux-musl' },
		{ unexpected: true },
	]) {
		const resolution = resolveBinding({
			host: supportedHost,
			sdkRelease: '0.12.0',
			loadAddon: () => addonReporting(skewed),
		});
		assert.equal(failureReason(resolution), 'contract_mismatch');
		assert.equal(resolution.kind === 'failed' ? resolution.failure.effects : '', 'none');
	}
	assert.equal(
		resolveBinding({
			host: supportedHost,
			sdkRelease: '0.12.0',
			loadAddon: () => addonReporting(exact),
		}).kind,
		'ready',
	);
});

await test('the installed host addon reports this release, revision and target', () => {
	const resolution = resolveBinding(installedBindingSources());
	assert.equal(
		resolution.kind,
		'ready',
		resolution.kind === 'failed' ? resolution.failure.message : '',
	);
});
