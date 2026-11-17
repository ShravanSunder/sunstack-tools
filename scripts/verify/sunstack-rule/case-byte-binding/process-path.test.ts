import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import operatingSystem from 'node:os';
import path from 'node:path';

import { test } from 'vitest';

import { binaryDigest, releasedTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';

test('process observer keeps isolated default PATH and permits only its explicit prefix', async () => {
	const root = filesystem.mkdtempSync(path.join(operatingSystem.tmpdir(), '2026-11-byte-path-'));
	const fixture = path.join(root, 'fixture');
	filesystem.mkdirSync(fixture);
	const binary = path.join(root, 'fake-sunstack');
	filesystem.writeFileSync(
		binary,
		`#!${process.execPath}\nif (process.argv[2] === '--version') console.log('sunstack test'); else console.log(process.env.PATH);\n`,
		{ mode: 0o700 },
	);
	const target = releasedTarget(binary, binaryDigest(binary), 'test');
	const observer = new RulebookProcess(target, path.join(root, 'evidence'));
	await observer.start();
	try {
		const ordinary = await observer.run({
			root: fixture,
			name: 'ordinary',
			arguments: ['inspect-path'],
			expectedExit: 0,
		});
		assert.equal(ordinary.stdout.toString('utf8').trim(), '/usr/bin:/bin');
		const controlled = path.join(root, 'owned-bin');
		filesystem.mkdirSync(controlled);
		const prefixed = await observer.run({
			root: fixture,
			name: 'prefixed',
			arguments: ['inspect-path'],
			executableDirectory: controlled,
			expectedExit: 0,
		});
		assert.equal(prefixed.stdout.toString('utf8').trim(), `${controlled}:/usr/bin:/bin`);
	} finally {
		await observer.close();
	}
});
