/** Create an owned Git forwarding executable to observe archive arguments. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

function compareRevisions(left: string, right: string): number {
	return left.localeCompare(right);
}

export function createArchiveObserver(
	directory: string,
	realGit: string,
): {
	readonly executableDirectory: string;
	readonly archiveLog: string;
} {
	assert.ok(path.isAbsolute(directory) && path.isAbsolute(realGit));
	filesystem.mkdirSync(directory, { recursive: false });
	const archiveLog = path.join(directory, 'archive.jsonl');
	const executable = path.join(directory, 'git');
	const source = `#!${process.execPath}\nimport fs from 'node:fs';\nimport { spawnSync } from 'node:child_process';\nconst args = process.argv.slice(2);\nif (args[0] === 'archive') fs.appendFileSync(${JSON.stringify(archiveLog)}, JSON.stringify(args) + '\\n');\nconst result = spawnSync(${JSON.stringify(realGit)}, args, { stdio: 'inherit' });\nif (result.error) { process.stderr.write(String(result.error)); process.exit(127); }\nif (result.signal) { process.kill(process.pid, result.signal); } else process.exit(result.status ?? 127);\n`;
	filesystem.writeFileSync(executable, source, { mode: 0o700, flag: 'wx' });
	return { executableDirectory: directory, archiveLog };
}

export function requireArchiveRevisions(log: string, expected: readonly string[]): void {
	const lines = filesystem.readFileSync(log, 'utf8').trim().split('\n');
	const observed = lines.map((line) => {
		const args: unknown = JSON.parse(line);
		assert.ok(Array.isArray(args) && args[0] === 'archive');
		const revision: unknown = args.at(-1);
		assert.ok(typeof revision === 'string');
		return revision;
	});
	assert.deepEqual(
		observed.toSorted(compareRevisions),
		[...expected].toSorted(compareRevisions),
		'Unexpected archived revisions',
	);
}
