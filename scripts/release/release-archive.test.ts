import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { gzipSync } from 'node:zlib';

import { test } from 'vitest';

import { readReleaseArchive } from './release-archive.ts';

function gnuMember(name: string, kind: string, data: Buffer): Buffer {
	const header = Buffer.alloc(512);
	header.write(name);
	header.write('0000755\0', 100);
	header.write(data.length.toString(8).padStart(11, '0') + '\0', 124);
	header.write(kind, 156);
	header.write('ustar ', 257);
	// GNU stores an access time here; this is not the POSIX path prefix.
	header.write('00000012345\0', 345);
	header.fill(32, 148, 156);
	header.write(
		header
			.reduce((sum, byte) => sum + byte, 0)
			.toString(8)
			.padStart(6, '0') + '\0 ',
		148,
	);
	return Buffer.concat([header, data, Buffer.alloc((512 - (data.length % 512)) % 512)]);
}

for (const format of ['ustar', 'pax'])
	test(`reads ${format} regular bytes and distinguishes directory entries`, (): void => {
		const root = mkdtempSync(path.join(tmpdir(), 'sunstack-archive-test-'));
		const directory = path.join(root, format === 'pax' ? 'a'.repeat(150) : 'release');
		mkdirSync(directory);
		writeFileSync(path.join(directory, 'sunstack'), 'literal executable bytes');
		const archive = path.join(root, 'release.tar.gz');
		execFileSync('/usr/bin/tar', [
			'--format',
			format,
			'-czf',
			archive,
			'-C',
			root,
			path.basename(directory),
		]);
		const members = readReleaseArchive(archive);
		const files = members.filter((member) => member.regular);
		assert.equal(files.length, 1);
		assert.equal(files[0]?.name, `${path.basename(directory)}/sunstack`);
		assert.deepEqual(files[0]?.data, Buffer.from('literal executable bytes'));
		assert.ok(members.some((member) => !member.regular));
	});

test('GNU long names and access-time fields preserve the regular member names', (): void => {
	const directory = mkdtempSync(path.join(tmpdir(), 'sunstack-gnu-archive-test-'));
	const archive = path.join(directory, 'release.tar.gz');
	const longName = `${'release-'.repeat(20)}/sunstack`;
	writeFileSync(
		archive,
		gzipSync(
			Buffer.concat([
				gnuMember('././@LongLink', 'L', Buffer.from(`${longName}\0`)),
				gnuMember('short', '0', Buffer.from('long-name executable')),
				gnuMember('sunstack', '0', Buffer.from('short-name executable')),
				Buffer.alloc(1024),
			]),
		),
	);
	assert.deepEqual(
		readReleaseArchive(archive).map((member) => ({
			name: member.name,
			data: member.data.toString(),
		})),
		[
			{ name: longName, data: 'long-name executable' },
			{ name: 'sunstack', data: 'short-name executable' },
		],
	);
});
