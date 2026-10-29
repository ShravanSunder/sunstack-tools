import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { test } from 'vitest';

import { checkElf, checkMacosDependencies, findArchives } from './check-linkage.ts';

function elfWithProgram(programType: number, dynamicTag = 0): Buffer {
	const binary = Buffer.alloc(160);
	Buffer.from([0x7f, 0x45, 0x4c, 0x46, 2, 1]).copy(binary);
	binary.writeBigUInt64LE(64n, 32);
	binary.writeUInt16LE(56, 54);
	binary.writeUInt16LE(1, 56);
	binary.writeUInt32LE(programType, 64);
	binary.writeBigUInt64LE(128n, 72);
	binary.writeBigUInt64LE(32n, 96);
	binary.writeBigInt64LE(BigInt(dynamicTag), 128);
	return binary;
}

test('Linux loader or needed library is refused but static ELF is allowed', (): void => {
	checkElf(elfWithProgram(1));
	checkElf(elfWithProgram(2));
	assert.throws(() => checkElf(elfWithProgram(3)), /PT_INTERP/);
	assert.throws(() => checkElf(elfWithProgram(2, 1)), /DT_NEEDED/);
	assert.throws(() => checkElf(Buffer.from('not an executable')), /ELF/);
});

test('macOS Homebrew dependency fails while OS dependencies pass', (): void => {
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

test('each job checks its own archive and cannot pass when it is missing', (): void => {
	const directory = mkdtempSync(path.join(tmpdir(), 'sunstack-linkage-test-'));
	const own = path.join(directory, 'sunstack-aarch64-unknown-linux-musl.tar.gz');
	writeFileSync(own, '');
	writeFileSync(path.join(directory, 'sunstack-aarch64-apple-darwin.tar.gz'), '');
	assert.deepEqual(findArchives(directory, ['aarch64-unknown-linux-musl']), [own]);
	assert.throws(() => findArchives(directory, ['x86_64-unknown-linux-musl']), /requested targets/);
});
