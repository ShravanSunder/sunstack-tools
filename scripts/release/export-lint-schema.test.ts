import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { test } from 'vitest';
import { z } from 'zod';

const exporter = fileURLToPath(new URL('./export-lint-schema.ts', import.meta.url));
const sdkPath = '/literal SDK path';

function schemaFixture(): { root: string; environment: NodeJS.ProcessEnv; receipt: string } {
	const root = mkdtempSync(path.join(tmpdir(), 'sunstack-schema-export-test-'));
	const binaries = path.join(root, 'bin');
	mkdirSync(binaries);
	const receipt = path.join(root, 'cargo.json');
	writeFileSync(
		path.join(binaries, 'cargo'),
		`#!${process.execPath}
const fs = require('node:fs');
fs.writeFileSync(${JSON.stringify(receipt)}, JSON.stringify({ arguments: process.argv.slice(2), sdkRoot: process.env.SDKROOT, compiler: process.env.CC, compilerFlags: process.env.CFLAGS }));
process.stdout.write('literal schema bytes\\n');
process.stderr.write('literal cargo diagnostic\\n');
process.exitCode = Number(process.env.SCHEMA_CARGO_EXIT);
`,
		{ mode: 0o755 },
	);
	writeFileSync(
		path.join(binaries, 'xcrun'),
		`#!${process.execPath}
process.stdout.write(${JSON.stringify(`${sdkPath}\n`)});
process.exitCode = Number(process.env.SCHEMA_SDK_EXIT);
`,
		{ mode: 0o755 },
	);
	return {
		root,
		receipt,
		environment: {
			...process.env,
			PATH: `${binaries}${path.delimiter}${process.env['PATH'] ?? ''}`,
			SCHEMA_CARGO_EXIT: '0',
			SCHEMA_SDK_EXIT: '0',
		},
	};
}

for (const exitCode of [0, 7])
	test(`schema export redirects only Cargo stdout and retains exit ${exitCode}`, (): void => {
		const fixture = schemaFixture();
		const result = spawnSync(process.execPath, [exporter], {
			cwd: fixture.root,
			env: { ...fixture.environment, SCHEMA_CARGO_EXIT: String(exitCode) },
			encoding: 'utf8',
		});
		assert.equal(result.status, exitCode);
		assert.equal(result.stdout, '');
		assert.equal(result.stderr, 'literal cargo diagnostic\n');
		assert.equal(
			readFileSync(path.join(fixture.root, 'target/sunstack.lint.schema.json'), 'utf8'),
			'literal schema bytes\n',
		);
		const receipt = z
			.object({
				arguments: z.array(z.string()),
				sdkRoot: z.string().optional(),
				compiler: z.string().optional(),
				compilerFlags: z.string().optional(),
			})
			.parse(JSON.parse(readFileSync(fixture.receipt, 'utf8')));
		assert.deepEqual(receipt.arguments, [
			'run',
			'--locked',
			'--quiet',
			'-p',
			'sunstack-lint',
			'--example',
			'export_schema',
		]);
		if (process.platform === 'darwin') {
			assert.equal(receipt.sdkRoot, sdkPath);
			assert.equal(receipt.compiler, '/usr/bin/clang');
			assert.equal(receipt.compilerFlags, `-isysroot ${sdkPath}`);
		}
	});

test.skipIf(process.platform !== 'darwin')(
	'SDK lookup failure stops before starting Cargo or creating output',
	(): void => {
		const fixture = schemaFixture();
		const result = spawnSync(process.execPath, [exporter], {
			cwd: fixture.root,
			env: { ...fixture.environment, SCHEMA_SDK_EXIT: '13' },
			encoding: 'utf8',
		});
		assert.equal(result.status, 13);
		assert.equal(result.stdout, '');
		assert.equal(result.stderr, '');
		assert.equal(existsSync(fixture.receipt), false);
		assert.equal(existsSync(path.join(fixture.root, 'target')), false);
	},
);
