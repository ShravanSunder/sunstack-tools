/** Read-only identity and byte/link guards for the commission's original frozen input. */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

export interface FrozenInput {
	readonly root: string;
	readonly head: string;
	readonly base: string;
	readonly trackedCount: number;
	readonly files: Record<string, string>;
	readonly gitAdmin: Record<string, string>;
}
function gitRead(root: string, arguments_: readonly string[]): string {
	const result = spawnSync(
		'git',
		['--no-optional-locks', '-c', 'core.fsmonitor=false', ...arguments_],
		{
			cwd: root,
			encoding: 'utf8',
			timeout: 10000,
			env: { ...process.env, GIT_OPTIONAL_LOCKS: '0' },
		},
	);
	assert.equal(result.status, 0, `Read-only Git query failed: ${result.stderr}`);
	return result.stdout.trim();
}
function digest(file: string): string {
	return createHash('sha256').update(filesystem.readFileSync(file)).digest('hex');
}
export function requireOutsideInput(input: string, output: string): void {
	const root = filesystem.realpathSync(input);
	const absolute = path.resolve(output);
	let ancestor = absolute;
	while (!filesystem.existsSync(ancestor)) {
		const parent = path.dirname(ancestor);
		assert.notEqual(parent, ancestor);
		ancestor = parent;
	}
	const resolved = path.resolve(
		filesystem.realpathSync(ancestor),
		path.relative(ancestor, absolute),
	);
	const relative = path.relative(root, resolved);
	assert.ok(
		relative === '..' || relative.startsWith('..' + path.sep) || path.isAbsolute(relative),
		'Evidence/config output would write inside the frozen input',
	);
}
export function snapshotFrozenInput(
	input: string,
	base: string,
	expectedHead: string,
): FrozenInput {
	const root = filesystem.realpathSync(input);
	assert.equal(filesystem.realpathSync(gitRead(root, ['rev-parse', '--show-toplevel'])), root);
	const head = gitRead(root, ['rev-parse', 'HEAD']);
	assert.equal(head, expectedHead, 'Frozen input HEAD differs from the declared original');
	assert.equal(gitRead(root, ['status', '--porcelain=v1']), '', 'Frozen input is not clean');
	const files: Record<string, string> = {};
	function visit(directory: string): void {
		for (const entry of filesystem.readdirSync(directory).toSorted()) {
			const file = path.join(directory, entry);
			const metadata = filesystem.lstatSync(file);
			if (entry === '.git' && metadata.isDirectory()) continue;
			const relative = path.relative(root, file);
			const mode = String(metadata.mode & 0o7777);
			if (metadata.isSymbolicLink())
				files[relative] = `link:${mode}:${JSON.stringify(filesystem.readlinkSync(file))}`;
			else if (metadata.isDirectory()) visit(file);
			else if (metadata.isFile()) files[relative] = `file:${mode}:${digest(file)}`;
			else throw new Error(`Unsupported frozen input entry: ${relative}`);
		}
	}
	visit(root);
	const gitAdmin: Record<string, string> = {};
	for (const name of ['HEAD', 'index', 'config']) {
		const file = path.resolve(root, gitRead(root, ['rev-parse', '--git-path', name]));
		gitAdmin[name] = filesystem.existsSync(file) ? digest(file) : 'absent';
	}
	return {
		root,
		head,
		base: gitRead(root, ['rev-parse', base]),
		trackedCount: gitRead(root, ['ls-files']).split('\n').filter(Boolean).length,
		files,
		gitAdmin,
	};
}
export function requireSameFrozenInput(before: FrozenInput, after: FrozenInput): void {
	assert.deepEqual(
		after,
		before,
		'Frozen original input or its Git identity changed; stop rather than repairing it',
	);
}
