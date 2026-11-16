import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import operatingSystem from 'node:os';
import path from 'node:path';

import { test } from 'vitest';

import { createArchiveObserver, requireArchiveRevisions } from './archive-observer.ts';

test('Git forwarding records archive attempts, delegates, and rejects extra revisions', () => {
	const root = filesystem.mkdtempSync(path.join(operatingSystem.tmpdir(), '2026-11-byte-archive-'));
	const observer = createArchiveObserver(path.join(root, 'bin'), '/usr/bin/git');
	const git = path.join(observer.executableDirectory, 'git');
	const version = spawnSync(git, ['--version'], { encoding: 'utf8' });
	assert.equal(version.status, 0);
	assert.match(version.stdout, /^git version /u);
	assert.equal(filesystem.existsSync(observer.archiveLog), false);
	const revision = '0123456789012345678901234567890123456789';
	const archive = spawnSync(
		git,
		['archive', '--format=tar', '--output', path.join(root, 'archive.tar'), revision],
		{ cwd: root, encoding: 'utf8' },
	);
	assert.notEqual(archive.status, 0);
	requireArchiveRevisions(observer.archiveLog, [revision]);
	assert.throws(() => requireArchiveRevisions(observer.archiveLog, []));
});
