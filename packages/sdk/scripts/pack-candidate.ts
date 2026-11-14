// Pack @sunstack/sdk and platform addon packages into a directory, with digests.
//
//   node scripts/pack-candidate.ts --out <absolute-directory> [--cell <key>]...
//
// Without --cell, packs this host's platform package. Each packed platform package
// must already contain its built addon.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, readdirSync } from 'node:fs';
import { isAbsolute, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

import { currentHostRuntime, platformCells, selectPlatformCell } from '../src/native-binding.ts';

const packageDirectory = fileURLToPath(new URL('../', import.meta.url));
const { values } = parseArgs({
	options: { out: { type: 'string' }, cell: { type: 'string', multiple: true } },
});
if (values.out === undefined || !isAbsolute(values.out))
	throw new Error('Pass --out <absolute-directory>.');
const output: string = values.out;
mkdirSync(output, { recursive: true });

const hostCell = selectPlatformCell(currentHostRuntime());
const keys = values.cell ?? (hostCell === null ? [] : [hostCell.key]);
const cells = keys.map((key) => {
	const cell = platformCells.find((candidate) => candidate.key === key);
	if (cell === undefined) throw new Error(`Unknown platform cell ${key}.`);
	return cell;
});

function pack(directory: string): string {
	const before = new Set(readdirSync(output));
	execFileSync('pnpm', ['pack', '--pack-destination', output], {
		cwd: directory,
		stdio: ['ignore', 'ignore', 'inherit'],
	});
	const created = readdirSync(output).filter((name) => !before.has(name) && name.endsWith('.tgz'));
	const [tarball] = created;
	if (tarball === undefined || created.length !== 1)
		throw new Error(`pnpm pack in ${directory} produced ${created.length} tarballs.`);
	return join(output, tarball);
}

const packed: { name: string; tarball: string; sha256: string }[] = [];
for (const cell of cells) {
	const directory = join(packageDirectory, 'npm', cell.key);
	const addon = join(directory, `sunstack-sdk.${cell.key}.node`);
	if (!existsSync(addon))
		throw new Error(`Build the ${cell.key} addon first: ${addon} is missing.`);
	const tarball = pack(directory);
	packed.push({
		name: `@sunstack/sdk-${cell.key}`,
		tarball,
		sha256: createHash('sha256').update(readFileSync(tarball)).digest('hex'),
	});
}
if (!existsSync(join(packageDirectory, 'dist', 'index.js')))
	throw new Error('Run build:typescript first.');
const sdkTarball = pack(packageDirectory);
packed.push({
	name: '@sunstack/sdk',
	tarball: sdkTarball,
	sha256: createHash('sha256').update(readFileSync(sdkTarball)).digest('hex'),
});
process.stdout.write(`${JSON.stringify(packed, null, 2)}\n`);
