// Build the Node-API addon for one platform cell into its npm/<cell> package.
//
//   node scripts/build-native.ts                 # this host's cell
//   node scripts/build-native.ts --target <rust-target-triple>
import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

import { currentHostRuntime, platformCells, selectPlatformCell } from '../src/native-binding.ts';

const packageDirectory = fileURLToPath(new URL('../', import.meta.url));
const repositoryDirectory = fileURLToPath(new URL('../../../', import.meta.url));

const { values } = parseArgs({
	options: { target: { type: 'string' }, profile: { type: 'string' } },
});
const cell =
	values.target === undefined
		? selectPlatformCell(currentHostRuntime())
		: (platformCells.find((candidate) => candidate.target === values.target) ?? null);
if (cell === null) {
	throw new Error(`No supported platform cell for ${values.target ?? 'this host'}.`);
}
const profile = values.profile ?? 'release';

function cargoEnvironment(): NodeJS.ProcessEnv {
	const environment: NodeJS.ProcessEnv = { ...process.env };
	if (cell?.libc === 'musl') {
		// A musl cdylib must link the C runtime dynamically to load into Node.
		environment['RUSTFLAGS'] =
			`${environment['RUSTFLAGS'] ?? ''} -C target-feature=-crt-static`.trim();
	}
	if (process.platform === 'darwin' && environment['SDKROOT'] === undefined) {
		const sdk = execFileSync('xcrun', ['--show-sdk-path'], { encoding: 'utf8' }).trim();
		environment['SDKROOT'] = sdk;
		environment['CC'] = '/usr/bin/clang';
		environment['CFLAGS'] = `-isysroot ${sdk}`;
	}
	return environment;
}

execFileSync(
	'cargo',
	[
		'build',
		'--locked',
		'-p',
		'sunstack-sdk-native',
		'--lib',
		'--profile',
		profile,
		'--target',
		cell.target,
	],
	{ cwd: repositoryDirectory, stdio: 'inherit', env: cargoEnvironment() },
);
const profileDirectory = profile === 'dev' ? 'debug' : profile;
const library =
	cell.platform === 'darwin' ? 'libsunstack_sdk_native.dylib' : 'libsunstack_sdk_native.so';
const built = join(repositoryDirectory, 'target', cell.target, profileDirectory, library);
if (!existsSync(built)) throw new Error(`Cargo produced no addon at ${built}`);
const destination = join(packageDirectory, 'npm', cell.key, `sunstack-sdk.${cell.key}.node`);
copyFileSync(built, destination);
process.stdout.write(`Built ${cell.target} addon: ${destination}\n`);
