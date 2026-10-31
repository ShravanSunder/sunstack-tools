/** Export the native lint schema with the same command-local Apple SDK setup as Cargo gates. */
import { execFileSync, spawnSync } from 'node:child_process';
import filesystem from 'node:fs';

export function exportLintSchema(): number {
	const environment = { ...process.env };
	if (process.platform === 'darwin') {
		const sdk = execFileSync('xcrun', ['--show-sdk-path'], { encoding: 'utf8' }).trim();
		environment['SDKROOT'] = sdk;
		environment['CC'] = '/usr/bin/clang';
		environment['CFLAGS'] = `-isysroot ${sdk}`;
	}
	filesystem.mkdirSync('target', { recursive: true });
	const output = filesystem.openSync('target/sunstack.lint.schema.json', 'w');
	try {
		const result = spawnSync(
			'cargo',
			['run', '--locked', '--quiet', '-p', 'sunstack-lint', '--example', 'export_schema'],
			{ env: environment, stdio: ['inherit', output, 'inherit'] },
		);
		if (result.error) throw result.error;
		if (result.signal) process.kill(process.pid, result.signal);
		return result.status ?? 1;
	} finally {
		filesystem.closeSync(output);
	}
}
if (import.meta.main) {
	try {
		process.exitCode = exportLintSchema();
	} catch (error) {
		process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
		process.exitCode = 1;
	}
}
