import assert from 'node:assert/strict';
import test from 'node:test';

import { checkNativeHeader, checkPackageMetadata } from '../scripts/check-packages.ts';
import type { PlatformCell } from '../src/native-binding.ts';

const linux: PlatformCell = {
	key: 'linux-x64-musl',
	platform: 'linux',
	arch: 'x64',
	libc: 'musl',
	target: 'x86_64-unknown-linux-musl',
};
const manifest = {
	name: '@sunstack/sdk-linux-x64-musl',
	version: '0.21.2',
	repository: { url: 'https://github.com/ShravanSunder/sunstack-tools' },
	engines: { node: '>=26.0.0' },
	os: ['linux'],
	cpu: ['x64'],
	libc: ['musl'],
	main: 'sunstack-sdk.linux-x64-musl.node',
	files: ['sunstack-sdk.linux-x64-musl.node'],
};
const files = ['package.json', 'sunstack-sdk.linux-x64-musl.node'];

void test('package gate accepts the advertised platform and rejects absent binaries or wrong metadata', () => {
	assert.doesNotThrow(() => checkPackageMetadata(manifest, files, '0.21.2', linux, []));
	for (const invalid of [
		{ ...manifest, libc: ['glibc'] },
		{ ...manifest, cpu: ['arm64'] },
		{ ...manifest, os: ['darwin'] },
		{ ...manifest, version: '0.21.1' },
	])
		assert.throws(() => checkPackageMetadata(invalid, files, '0.21.2', linux, []));
	assert.throws(() => checkPackageMetadata(manifest, ['package.json'], '0.21.2', linux, []));
	assert.throws(() =>
		checkPackageMetadata(manifest, [...files, 'unintended.txt'], '0.21.2', linux, []),
	);
});

void test('SDK package gate requires six exact-version optional dependencies and public entry points', () => {
	const sdkFiles = ['package.json', 'README.md', 'dist/index.js', 'dist/index.d.ts'];
	const sdk = {
		name: '@sunstack/sdk',
		version: '0.21.2',
		repository: manifest.repository,
		engines: manifest.engines,
		type: 'module',
		exports: {
			'.': { types: './dist/index.d.ts', default: './dist/index.js' },
			'./package.json': './package.json',
		},
		optionalDependencies: {
			'@sunstack/sdk-darwin-arm64': '0.21.2',
			'@sunstack/sdk-darwin-x64': '0.21.2',
			'@sunstack/sdk-linux-arm64-gnu': '0.21.2',
			'@sunstack/sdk-linux-arm64-musl': '0.21.2',
			'@sunstack/sdk-linux-x64-gnu': '0.21.2',
			'@sunstack/sdk-linux-x64-musl': '0.21.2',
		},
	};
	assert.doesNotThrow(() => checkPackageMetadata(sdk, sdkFiles, '0.21.2', null, sdkFiles));
	assert.throws(() =>
		checkPackageMetadata(
			{
				...sdk,
				optionalDependencies: {
					...sdk.optionalDependencies,
					'@sunstack/sdk-darwin-arm64': 'workspace:*',
				},
			},
			sdkFiles,
			'0.21.2',
			null,
			sdkFiles,
		),
	);
	assert.throws(() =>
		checkPackageMetadata({ ...sdk, exports: {} }, sdkFiles, '0.21.2', null, sdkFiles),
	);
	assert.throws(() =>
		checkPackageMetadata(sdk, [...sdkFiles, 'src/private.ts'], '0.21.2', null, sdkFiles),
	);
});

void test('binary gate rejects truncated libraries and architecture swaps', () => {
	const elf = Buffer.alloc(80);
	elf.write('7f454c46', 0, 'hex');
	elf[4] = 2;
	elf[5] = 1;
	elf.writeUInt16LE(3, 16);
	elf.writeUInt16LE(62, 18);
	assert.doesNotThrow(() => checkNativeHeader(elf, linux));
	assert.throws(() => checkNativeHeader(Buffer.alloc(0), linux));
	elf.writeUInt16LE(183, 18);
	assert.throws(() => checkNativeHeader(elf, linux));
	const darwin: PlatformCell = {
		key: 'darwin-arm64',
		platform: 'darwin',
		arch: 'arm64',
		libc: null,
		target: 'aarch64-apple-darwin',
	};
	const macho = Buffer.alloc(80);
	macho.writeUInt32LE(0xfeedfacf, 0);
	macho.writeUInt32LE(0x100000c, 4);
	macho.writeUInt32LE(6, 12);
	assert.doesNotThrow(() => checkNativeHeader(macho, darwin));
	macho.writeUInt32LE(0x1000007, 4);
	assert.throws(() => checkNativeHeader(macho, darwin));
});
