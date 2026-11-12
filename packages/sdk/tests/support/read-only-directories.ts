// Run a call with test-owned directories, and everything inside them, read-only, so any
// write the call attempts there fails instead of being cleaned up before it returns.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { chmodSync, closeSync, lstatSync, openSync, readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';

import { ownedDirectory } from './sdk-fixtures.ts';

/** Every entry under a directory with its kind, mode and content digest. */
function treeInventory(directory: string): string[] {
	const ownMode = `./ ${(lstatSync(directory).mode & 0o7777).toString(8)}`;
	return readdirSync(directory, { recursive: true, withFileTypes: true })
		.map((entry) => join(entry.parentPath, entry.name))
		.map((path) => {
			const status = lstatSync(path);
			const mode = (status.mode & 0o7777).toString(8);
			if (status.isFile()) {
				return `${relative(directory, path)} ${mode} ${createHash('sha256').update(readFileSync(path)).digest('hex')}`;
			}
			return `${relative(directory, path)}${status.isDirectory() ? '/' : '@'} ${mode}`;
		})
		.concat(ownMode)
		.toSorted();
}

/** The directory and every file and directory below it; symbolic links are refused, never followed. */
function ownedEntries(directory: string): { directories: string[]; files: string[] } {
	const directories = [directory];
	const files: string[] = [];
	for (const entry of readdirSync(directory, { recursive: true, withFileTypes: true })) {
		const path = join(entry.parentPath, entry.name);
		if (entry.isSymbolicLink())
			throw new Error(`refusing to lock through a symbolic link: ${path}`);
		if (entry.isDirectory()) directories.push(path);
		else if (entry.isFile()) files.push(path);
		else throw new Error(`refusing to lock a special file: ${path}`);
	}
	return { directories, files };
}

function refusedWrite(path: string, flags: string): boolean {
	try {
		closeSync(openSync(path, flags));
		return false;
	} catch (error) {
		return error instanceof Error && 'code' in error && error.code === 'EACCES';
	}
}

/**
 * Run `call` with the temporary directory and working directory moved to an owned scratch
 * directory, and that scratch plus `directories` locked: directories 0500, files 0400.
 * Live controls first show that creating a file in every locked directory and opening every
 * locked file for writing are refused. Original modes, cwd and TMPDIR are restored even when
 * locking or the call fails, and the trees must then be unchanged.
 */
export async function withoutWritableDirectories<TResult>(
	directories: readonly string[],
	call: () => Promise<TResult>,
): Promise<TResult> {
	const scratch = ownedDirectory('scratch-tmp');
	const roots = [scratch.path, ...directories];
	const previousTemporary = process.env['TMPDIR'];
	const previousDirectory = process.cwd();
	const before = roots.map(treeInventory);
	const restoreModes: (readonly [string, number])[] = [];
	try {
		const entries = roots.map(ownedEntries);
		for (const { directories: lockedDirectories, files } of entries) {
			for (const path of [...files, ...lockedDirectories.toReversed()]) {
				restoreModes.push([path, lstatSync(path).mode & 0o7777]);
				chmodSync(path, lockedDirectories.includes(path) ? 0o500 : 0o400);
			}
		}
		for (const { directories: lockedDirectories, files } of entries) {
			for (const directory of lockedDirectories) {
				assert.ok(
					refusedWrite(join(directory, '.write-probe'), 'wx'),
					`creating in ${directory} is refused`,
				);
			}
			for (const file of files) assert.ok(refusedWrite(file, 'r+'), `writing ${file} is refused`);
		}
		process.env['TMPDIR'] = scratch.path;
		process.chdir(scratch.path);
		return await call();
	} finally {
		process.chdir(previousDirectory);
		if (previousTemporary === undefined) delete process.env['TMPDIR'];
		else process.env['TMPDIR'] = previousTemporary;
		for (const [path, mode] of restoreModes.toReversed()) chmodSync(path, mode);
		assert.deepEqual(
			roots.map(treeInventory),
			before,
			'no entry appeared, changed or kept a new mode',
		);
		scratch.remove();
	}
}
