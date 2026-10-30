import { spawnSync, type SpawnSyncReturns } from 'node:child_process';
import { closeSync, mkdirSync, openSync } from 'node:fs';
import { constants } from 'node:os';

function commandExit(result: SpawnSyncReturns<string | Buffer>, command: string): number {
	if (result.error) {
		process.stderr.write(`${command}: ${result.error.message}\n`);
		return 'code' in result.error && result.error.code === 'ENOENT' ? 127 : 126;
	}
	return result.status ?? (result.signal ? 128 + constants.signals[result.signal] : 1);
}

export function exportLintSchema(): number {
	let environment = process.env;
	if (process.platform === 'darwin') {
		const sdk = spawnSync('xcrun', ['--show-sdk-path'], {
			encoding: 'utf8',
			stdio: ['inherit', 'pipe', 'inherit'],
		});
		if (sdk.status !== 0) return commandExit(sdk, 'xcrun');
		const sdkRoot = sdk.stdout.replace(/\n+$/, '');
		environment = {
			...environment,
			SDKROOT: sdkRoot,
			CFLAGS: `-isysroot ${sdkRoot}`,
		};
		environment['CC'] = '/usr/bin/clang';
	}
	mkdirSync('target', { recursive: true });
	const output = openSync('target/sunstack.lint.schema.json', 'w');
	try {
		const cargo = spawnSync(
			'cargo',
			['run', '--locked', '--quiet', '-p', 'sunstack-lint', '--example', 'export_schema'],
			{
				env: environment,
				stdio: ['inherit', output, 'inherit'],
			},
		);
		return commandExit(cargo, 'cargo');
	} finally {
		closeSync(output);
	}
}

if (import.meta.main) process.exitCode = exportLintSchema();
