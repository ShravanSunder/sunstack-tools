// Validate the exact tarballs consumed by install CI and release publishing.
//   node packages/sdk/scripts/check-packages.ts --tarballs <directory> [--tag vX.Y.Z]
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

import type { PlatformCell } from '../src/native-binding.ts';
import { platformCells } from '../src/native-binding.ts';

const packageDirectory = fileURLToPath(new URL('../', import.meta.url));

function objectValue(value: unknown): Record<string, unknown> {
	assert.ok(typeof value === 'object' && value !== null && !Array.isArray(value));
	return Object.fromEntries(Object.entries(value));
}

/** Reject a missing binary, wrong platform, or unresolved workspace dependency. */
export function checkPackageMetadata(
	manifest: unknown,
	files: readonly string[],
	version: string,
	cell: PlatformCell | null,
	expectedSdkFiles: readonly string[],
): void {
	const metadata = objectValue(manifest);
	assert.equal(metadata['name'], cell === null ? '@sunstack/sdk' : `@sunstack/sdk-${cell.key}`);
	assert.equal(metadata['version'], version);
	assert.equal(metadata['private'], undefined);
	assert.equal(
		objectValue(metadata['repository'])['url'],
		'https://github.com/ShravanSunder/sunstack-tools',
	);
	assert.deepEqual(metadata['engines'], { node: '>=26.0.0' });
	assert.deepEqual(metadata['os'], cell === null ? undefined : [cell.platform]);
	assert.deepEqual(metadata['cpu'], cell === null ? undefined : [cell.arch]);
	assert.deepEqual(
		metadata['libc'],
		cell?.libc === null || cell === null ? undefined : [cell.libc],
	);
	if (cell === null) {
		assert.deepEqual(files.toSorted(), [...expectedSdkFiles].toSorted());
		assert.deepEqual(
			metadata['optionalDependencies'],
			Object.fromEntries(
				platformCells.map((platform) => [`@sunstack/sdk-${platform.key}`, version]),
			),
		);
		assert.equal(metadata['type'], 'module');
		assert.deepEqual(metadata['exports'], {
			'.': { types: './dist/index.d.ts', default: './dist/index.js' },
			'./package.json': './package.json',
		});
	} else {
		const binary = `sunstack-sdk.${cell.key}.node`;
		assert.deepEqual(files.toSorted(), ['package.json', binary]);
		assert.equal(metadata['main'], binary);
		assert.deepEqual(metadata['files'], [binary]);
	}
}

/** The archive must contain a native library for the advertised architecture. */
export function checkNativeHeader(binary: Buffer, cell: PlatformCell): void {
	assert.ok(binary.length > 64, `${cell.key}: native library is empty or truncated`);
	if (cell.platform === 'darwin') {
		assert.equal(binary.readUInt32LE(0), 0xfeedfacf, '64-bit Mach-O');
		assert.equal(
			binary.readUInt32LE(4),
			cell.arch === 'arm64' ? 0x100000c : 0x1000007,
			'Mach-O CPU',
		);
		assert.equal(binary.readUInt32LE(12), 6, 'Mach-O dynamic library');
	} else {
		assert.equal(binary.subarray(0, 4).toString('hex'), '7f454c46', 'ELF');
		assert.equal(binary[4], 2, '64-bit ELF');
		assert.equal(binary[5], 1, 'little-endian ELF');
		assert.equal(binary.readUInt16LE(16), 3, 'ELF shared library');
		assert.equal(binary.readUInt16LE(18), cell.arch === 'arm64' ? 183 : 62, 'ELF CPU');
	}
}

function checkTarballs(): void {
	const { values } = parseArgs({
		options: { tarballs: { type: 'string' }, tag: { type: 'string' } },
	});
	assert.ok(values.tarballs, 'Pass --tarballs <directory>.');
	const directory = resolve(values.tarballs);
	const metadata = objectValue(
		JSON.parse(readFileSync(join(packageDirectory, 'package.json'), 'utf8')),
	);
	const version = metadata['version'];
	assert.equal(typeof version, 'string');
	assert.ok(typeof version === 'string');
	const cargoManifest = readFileSync(join(packageDirectory, '../../Cargo.toml'), 'utf8');
	assert.equal(cargoManifest.match(/^version = "([^"]+)"/mu)?.[1], version, 'CLI and SDK version');
	if (values.tag !== undefined)
		assert.equal(values.tag, `v${version}`, 'release tag and SDK version');
	const sourceFiles = readdirSync(join(packageDirectory, 'src'), {
		recursive: true,
		encoding: 'utf8',
	});
	const expectedSdkFiles = [
		'README.md',
		'package.json',
		...sourceFiles.flatMap((file) =>
			file.endsWith('.ts')
				? [`dist/${file.slice(0, -3)}.js`, `dist/${file.slice(0, -3)}.d.ts`]
				: [],
		),
		...readdirSync(join(packageDirectory, 'examples')).map((file) => `examples/${file}`),
	];
	const expectedTarballs = [
		...platformCells.map((cell) => `sunstack-sdk-${cell.key}-${version}.tgz`),
		`sunstack-sdk-${version}.tgz`,
	];
	assert.deepEqual(
		readdirSync(directory)
			.filter((name) => name.endsWith('.tgz'))
			.toSorted(),
		expectedTarballs.toSorted(),
		'all seven packages, exactly once',
	);
	const reports = [...platformCells, null].map((cell) => {
		const filename =
			cell === null ? `sunstack-sdk-${version}.tgz` : `sunstack-sdk-${cell.key}-${version}.tgz`;
		const tarball = join(directory, filename);
		const members = execFileSync('tar', ['-tzf', tarball], { encoding: 'utf8' }).trim().split('\n');
		assert.ok(members.every((member) => member.startsWith('package/')));
		const files = members.map((member) => member.slice('package/'.length));
		const manifest: unknown = JSON.parse(
			execFileSync('tar', ['-xOzf', tarball, 'package/package.json'], { encoding: 'utf8' }),
		);
		checkPackageMetadata(manifest, files, version, cell, expectedSdkFiles);
		let nativeSha256: string | null = null;
		if (cell !== null) {
			const binary = execFileSync(
				'tar',
				['-xOzf', tarball, `package/sunstack-sdk.${cell.key}.node`],
				{ maxBuffer: 128 * 1024 * 1024 },
			);
			checkNativeHeader(binary, cell);
			nativeSha256 = createHash('sha256').update(binary).digest('hex');
		}
		const dryRun: unknown = JSON.parse(
			execFileSync(
				'npm',
				[
					'publish',
					'--dry-run',
					'--json',
					'--access',
					'public',
					'--ignore-scripts',
					'--registry=https://registry.npmjs.org',
					tarball,
				],
				{ encoding: 'utf8', maxBuffer: 8 * 1024 * 1024 },
			),
		);
		const dryRunPackages = objectValue(dryRun);
		const packageName = objectValue(manifest)['name'];
		assert.ok(typeof packageName === 'string');
		assert.deepEqual(Object.keys(dryRunPackages), [packageName]);
		const published = objectValue(dryRunPackages[packageName]);
		assert.equal(published['name'], objectValue(manifest)['name']);
		assert.equal(published['version'], version);
		const publishedFiles = published['files'];
		assert.ok(Array.isArray(publishedFiles));
		assert.deepEqual(
			publishedFiles
				.map((file: unknown) => {
					const path = objectValue(file)['path'];
					assert.ok(typeof path === 'string');
					return path;
				})
				.toSorted(),
			files.toSorted(),
		);
		return {
			filename,
			sha256: createHash('sha256').update(readFileSync(tarball)).digest('hex'),
			manifest,
			files: files.toSorted(),
			native_sha256: nativeSha256,
			dry_run: published,
		};
	});
	process.stdout.write(`${JSON.stringify(reports, null, 2)}\n`);
}

if (import.meta.main) checkTarballs();
