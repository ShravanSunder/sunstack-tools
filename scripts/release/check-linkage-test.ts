import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import filesystem from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';

import { checkArchive, checkElf, checkMacosDependencies, findArchives } from './check-linkage.ts';

function elfWithProgram(programType: number, dynamicTag = 0): Buffer {
	const binary = Buffer.alloc(160);
	binary.set([0x7f, 0x45, 0x4c, 0x46, 2, 1]);
	binary.writeBigUInt64LE(64n, 32);
	binary.writeUInt16LE(56, 54);
	binary.writeUInt16LE(1, 56);
	binary.writeUInt32LE(programType, 64);
	binary.writeBigUInt64LE(128n, 72);
	binary.writeBigUInt64LE(32n, 96);
	binary.writeBigInt64LE(BigInt(dynamicTag), 128);
	return binary;
}

await test('Linux loader or needed library is refused but static ELF is allowed', (): void => {
	checkElf(elfWithProgram(1));
	checkElf(elfWithProgram(2));
	assert.throws(() => checkElf(elfWithProgram(3)), /PT_INTERP/);
	assert.throws(() => checkElf(elfWithProgram(2, 1)), /DT_NEEDED/);
	assert.throws(() => checkElf(Buffer.from('not an executable')), /ELF/);
});

await test('macOS Homebrew dependency fails while OS dependencies pass', (): void => {
	const allowed =
		'sunstack:\n\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0)\n\t/System/Library/Frameworks/Security.framework/Versions/A/Security (compatibility version 1.0.0)\n';
	checkMacosDependencies(allowed);
	assert.throws(
		() =>
			checkMacosDependencies(
				`${allowed}\t/opt/homebrew/opt/libgit2/lib/libgit2.dylib (compatibility version 1.9.0)\n`,
			),
		/non-OS/,
	);
	assert.throws(() => checkMacosDependencies('sunstack:\n'), /no macOS libraries/);
});

await test('each job checks its own archive and cannot pass when it is missing', (context): void => {
	const directory = filesystem.mkdtempSync(path.join(tmpdir(), 'sunstack-linkage-test-'));
	context.after(() => filesystem.rmSync(directory, { recursive: true, force: true }));
	const own = path.join(directory, 'sunstack-aarch64-unknown-linux-musl.tar.gz');
	filesystem.writeFileSync(own, '');
	filesystem.writeFileSync(path.join(directory, 'sunstack-aarch64-apple-darwin.tar.gz'), '');
	assert.deepEqual(findArchives(directory, ['aarch64-unknown-linux-musl']), [own]);
	assert.throws(() => findArchives(directory, ['x86_64-unknown-linux-musl']), /requested targets/);
});

await test('real tar member types exclude directories and links and preserve a PAX long-name executable', (context): void => {
	const directory = filesystem.mkdtempSync(path.join(tmpdir(), 'sunstack-linkage-tar-test-'));
	context.after(() => filesystem.rmSync(directory, { recursive: true, force: true }));
	const stage = path.join(directory, 'stage');
	const nested = path.join(stage, 'x'.repeat(120));
	filesystem.mkdirSync(nested, { recursive: true });
	filesystem.writeFileSync(path.join(nested, 'sunstack'), elfWithProgram(1));
	filesystem.mkdirSync(path.join(stage, 'sunstack'));
	filesystem.symlinkSync(path.join(nested, 'sunstack'), path.join(stage, 'sunstack', 'sunstack'));
	filesystem.writeFileSync(path.join(stage, 'sunstack '), 'different member name');
	const archive = path.join(directory, 'sunstack-aarch64-unknown-linux-musl.tar.gz');
	const archivedPaths = [
		path.relative(stage, path.join(nested, 'sunstack')),
		'sunstack',
		'sunstack ',
	];
	execFileSync('tar', ['--format=pax', '-czf', archive, '-C', stage, ...archivedPaths]);
	checkArchive(archive);
	filesystem.writeFileSync(path.join(stage, 'sunstack', 'second'), elfWithProgram(1));
	filesystem.renameSync(
		path.join(stage, 'sunstack', 'sunstack'),
		path.join(stage, 'sunstack', 'link'),
	);
	filesystem.renameSync(
		path.join(stage, 'sunstack', 'second'),
		path.join(stage, 'sunstack', 'sunstack'),
	);
	execFileSync('tar', ['--format=pax', '-czf', archive, '-C', stage, ...archivedPaths]);
	assert.throws(() => checkArchive(archive), /exactly one/);
});
