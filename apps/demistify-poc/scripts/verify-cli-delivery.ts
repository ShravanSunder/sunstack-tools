/** Verify staged or separately authorized hosted Demistify CLI delivery through package managers. */
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cp, mkdtemp, readFile, readdir, realpath, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

import { z } from 'zod';

const usage = `Usage: node apps/demistify-poc/scripts/verify-cli-delivery.ts --stage <release-directory> --major <number>
       node apps/demistify-poc/scripts/verify-cli-delivery.ts --stage <release-directory> --major <number> --run <guide-or-loopback-origin> --input <collection>
       node apps/demistify-poc/scripts/verify-cli-delivery.ts --help

Read the staged manifest, published guide examples and archive bytes. This
read-only preflight checks that the advertised archive agrees with the manifest;
it does not run pnpm or npx, establish cache selection or prove a release.
With --run, use an admitted guide or loopback origin and owned input copies for real
pnpm and npx commands plus instrumented identity companions. Keep its output
directory. A loopback run is staged evidence, not a hosted release observation.
`;
const argumentsList = process.argv.slice(2);
if (argumentsList.length === 1 && argumentsList[0] === '--help') {
	process.stdout.write(usage);
} else if (
	(argumentsList.length !== 4 && argumentsList.length !== 8) ||
	(argumentsList.length === 8 &&
		(argumentsList[4] !== '--run' ||
			!argumentsList[5] ||
			argumentsList[6] !== '--input' ||
			!argumentsList[7])) ||
	argumentsList[0] !== '--stage' ||
	argumentsList[2] !== '--major' ||
	!argumentsList[1] ||
	!argumentsList[3] ||
	!/^[1-9]\d*$/.test(argumentsList[3])
) {
	process.stderr.write(usage);
	process.exitCode = 2;
} else {
	const manifestSchema = z.object({
		version: z.string(),
		major: z.number().int().positive(),
		sourceCommit: z.string(),
		files: z.array(z.object({ path: z.string(), sha256: z.string(), bytes: z.number() })),
	});
	const stageDirectory = resolve(argumentsList[1]);
	const majorDirectory = join(stageDirectory, `v${argumentsList[3]}`);
	const manifest = manifestSchema.parse(
		JSON.parse(await readFile(join(majorDirectory, 'release.json'), 'utf8')),
	);
	if (manifest.major !== Number(argumentsList[3]))
		throw new Error('Manifest major differs from requested major.');
	const guideDirectory = join(majorDirectory, 'authoring');
	const entries = await readdir(guideDirectory, { recursive: true, withFileTypes: true });
	const guideFiles = entries.filter(
		(entry): boolean => entry.isFile() && entry.name.endsWith('.md'),
	);
	const examples: { file: string; line: string; archive: string }[] = [];
	const guideContents = await Promise.all(
		guideFiles.map(async (entry) => {
			const file = join(entry.parentPath, entry.name);
			return { file, lines: (await readFile(file, 'utf8')).split('\n') };
		}),
	);
	for (const { file, lines } of guideContents) {
		for (const line of lines) {
			if (!/\b(?:pnpm dlx|npx)\b/.test(line) || !line.includes('demistify-cli')) continue;
			const match = line.match(/demistify-cli-[a-f0-9]{64}\.tgz/g);
			if (!match || match.length !== 1 || !match[0])
				throw new Error(`Unresolved CLI example in ${file}: ${line}`);
			examples.push({ file, line, archive: match[0] });
		}
	}
	if (examples.length === 0) throw new Error('No CLI examples found in staged authoring.');
	const archiveNames = new Set(examples.map((example): string => example.archive));
	if (archiveNames.size !== 1)
		throw new Error('Staged guide examples do not use one matching archive.');
	const archiveName = examples[0]?.archive;
	if (!archiveName) throw new Error('No archive selected.');
	const hostedOrigins = new Set<string>();
	for (const example of examples) {
		const hosted = example.line.match(/https:\/\/[^\s]+\.tgz/)?.[0];
		if (hosted) {
			const url = new URL(hosted);
			if (url.pathname !== `/v${manifest.major}/${archiveName}` || url.search || url.hash)
				throw new Error(`Wrong hosted path in ${example.file}: ${example.line}`);
			hostedOrigins.add(url.origin);
		} else if (!example.line.includes(`"$PWD/trail-guide/demistify/${archiveName}"`)) {
			throw new Error(`Wrong local binding path in ${example.file}: ${example.line}`);
		}
	}
	if (hostedOrigins.size !== 1) throw new Error('Hosted guide examples disagree on origin.');
	const archive = await readFile(join(majorDirectory, archiveName));
	const digest = createHash('sha256').update(archive).digest('hex');
	const record = manifest.files.find((file): boolean => file.path === archiveName);
	if (
		!record ||
		record.sha256 !== digest ||
		record.bytes !== archive.byteLength ||
		archiveName !== `demistify-cli-${digest}.tgz`
	)
		throw new Error('Advertised archive does not match its manifest entry and bytes.');
	const summary = {
		stageDirectory,
		version: manifest.version,
		sourceCommit: manifest.sourceCommit,
		archiveName,
		sha256: digest,
		bytes: archive.byteLength,
		examples,
	};
	process.stdout.write(`${JSON.stringify(summary, null, 2)}\n`);
	if (argumentsList.length === 4) {
		process.stderr.write(
			'Preflight only: package-manager execution and identity are unverified.\n',
		);
		process.exitCode = 2;
	} else {
		const origin = new URL(argumentsList[5] ?? '');
		if (origin.pathname !== '/' || origin.search || origin.hash)
			throw new Error('Expected an origin without a path or query.');
		const inputDirectory = resolve(argumentsList[7] ?? '');
		const publicExample = examples.find((example) => example.line.includes('pnpm dlx https://'));
		const publicUrl = publicExample?.line.match(
			/https:\/\/[^\s]+demistify-cli-[a-f0-9]{64}\.tgz/,
		)?.[0];
		if (!publicUrl) throw new Error('Staged guide has no hosted pnpm CLI URL.');
		const sourceUrl = new URL(publicUrl);
		const loopback = origin.protocol === 'http:' && origin.hostname === '127.0.0.1';
		if (!loopback && origin.origin !== sourceUrl.origin)
			throw new Error('Origin must be the guide HTTPS origin or admitted loopback.');
		const localUrl = new URL(sourceUrl.pathname, origin).href;
		const manifestResponse = await fetch(new URL(`/v${manifest.major}/release.json`, origin));
		if (
			!manifestResponse.ok ||
			!Buffer.from(await manifestResponse.arrayBuffer()).equals(
				await readFile(join(majorDirectory, 'release.json')),
			)
		)
			throw new Error('HTTP manifest differs from frozen stage.');
		const response = await fetch(localUrl);
		if (!response.ok) throw new Error(`Staged archive HTTP ${response.status}.`);
		const fetched = Buffer.from(await response.arrayBuffer());
		if (!fetched.equals(archive)) throw new Error('HTTP archive differs from staged archive.');
		const work = await mkdtemp(join(tmpdir(), '2026-11-cli-delivery-'));
		const collection = join(work, 'trail-guide');
		await cp(inputDirectory, collection, { recursive: true, errorOnExist: true });
		await cp(majorDirectory, join(collection, 'demistify'), {
			recursive: true,
			errorOnExist: true,
		});
		const pages = [
			'requirements.html',
			'specification.html',
			'program-design.html',
			'verification.html',
			'execution.html',
		].map((name): string => join(collection, name));
		const observerFile = join(work, 'observe-cli.cjs');
		const observer = `const fs = require('node:fs'); const crypto = require('node:crypto'); const path = require('node:path');\nconst entry = process.argv[1]; const real = entry && fs.existsSync(entry) ? fs.realpathSync(entry) : ''; if (real && path.basename(real) === 'demistify.js') { const metadata = JSON.parse(fs.readFileSync(path.join(path.dirname(real), 'package.json'), 'utf8')); const stat = fs.statSync(real); fs.appendFileSync(${JSON.stringify(join(work, 'identity.jsonl'))}, JSON.stringify({pid:process.pid,argv1:entry,realpath:real,inode:stat.ino,size:stat.size,mtimeMs:stat.mtimeMs,sha256:crypto.createHash('sha256').update(fs.readFileSync(real)).digest('hex'),name:metadata.name,version:metadata.version})+'\\n'); }\n`;
		await writeFile(observerFile, observer, { flag: 'wx' });
		if (process.env['NODE_OPTIONS'])
			throw new Error('Existing NODE_OPTIONS must be reviewed before observer injection.');
		function invoke(label: string, executable: string, args: string[], observed = false): void {
			const env = {
				...process.env,
				...(observed ? { NODE_OPTIONS: `--require=${observerFile}` } : {}),
			};
			const commandDirectory = label.startsWith('pnpm-local-') ? work : process.cwd();
			const result = spawnSync(executable, args, {
				cwd: commandDirectory,
				env,
				encoding: 'utf8',
				timeout: 120_000,
			});
			const receipt = {
				executable,
				args,
				cwd: commandDirectory,
				observer: observed,
				status: result.status,
				signal: result.signal,
				error: result.error?.message,
				stdout: result.stdout,
				stderr: result.stderr,
			};
			process.stdout.write(`${JSON.stringify({ label, ...receipt })}\n`);
			if (result.status !== 0)
				throw new Error(`${label} failed; retain its printed receipt and ${work}.`);
		}
		const cachePathResult = spawnSync('pnpm', ['cache', 'path'], {
			cwd: process.cwd(),
			encoding: 'utf8',
		});
		if (cachePathResult.status !== 0) throw new Error('Cannot determine pnpm cache path.');
		const cacheDirectory = join(cachePathResult.stdout.trim(), 'dlx');
		const oldSpec = new URL(`/v${manifest.major}/demistify-cli.tgz`, sourceUrl.origin).href;
		async function inventoryCache(): Promise<{
			complete: boolean;
			matches: { spec: string; target: string; lockContainsSpec: boolean }[];
		}> {
			const folders = await readdir(cacheDirectory, { withFileTypes: true });
			let complete = true;
			const matches: { spec: string; target: string; lockContainsSpec: boolean }[] = [];
			await Promise.all(
				folders
					.filter((folder): boolean => folder.isDirectory())
					.map(async (folder) => {
						const packageRoot = join(cacheDirectory, folder.name, 'pkg');
						try {
							const body: unknown = JSON.parse(
								await readFile(join(packageRoot, 'package.json'), 'utf8'),
							);
							const parsed = z
								.object({ dependencies: z.record(z.string(), z.string()).optional() })
								.safeParse(body);
							const spec = parsed.success
								? parsed.data.dependencies?.['@sunstack/demistify-cli']
								: undefined;
							if (spec !== localUrl && spec !== oldSpec) return;
							const lock = await readFile(join(packageRoot, 'pnpm-lock.yaml'), 'utf8');
							matches.push({
								spec,
								target: await realpath(packageRoot),
								lockContainsSpec: lock.includes(spec),
							});
						} catch (error) {
							if (!(error instanceof Error && 'code' in error && error.code === 'ENOENT'))
								complete = false;
						}
					}),
			);
			return {
				complete,
				matches: matches.toSorted((left, right): number => left.target.localeCompare(right.target)),
			};
		}
		const beforeCache = await inventoryCache();
		await writeFile(join(work, 'cache-before.json'), JSON.stringify(beforeCache, null, 2));
		process.stdout.write(`${JSON.stringify({ cacheDirectory, cacheBefore: beforeCache })}\n`);
		process.stdout.write(`Evidence directory: ${work}\n`);
		for (const executable of ['node', 'pnpm', 'npx']) {
			const result = spawnSync(executable, ['--version'], { cwd: process.cwd(), encoding: 'utf8' });
			if (result.status !== 0) throw new Error(`${executable} version unavailable.`);
			process.stdout.write(`${JSON.stringify({ executable, version: result.stdout.trim() })}\n`);
		}
		invoke('pnpm-first-prepare', 'pnpm', ['dlx', localUrl, 'prepare', collection]);
		invoke('pnpm-first-check', 'pnpm', ['dlx', localUrl, 'check', ...pages]);
		invoke('pnpm-repeat-prepare', 'pnpm', ['dlx', localUrl, 'prepare', collection]);
		invoke('pnpm-repeat-check', 'pnpm', ['dlx', localUrl, 'check', ...pages]);
		invoke('pnpm-observed-prepare', 'pnpm', ['dlx', localUrl, 'prepare', collection], true);
		const firstReceipt = await readFile(join(work, 'identity.jsonl'), 'utf8');
		if (firstReceipt.trim().split('\n').length !== 1)
			throw new Error('Observer did not identify exactly one CLI child.');
		invoke('pnpm-observed-repeat', 'pnpm', ['dlx', localUrl, 'prepare', collection], true);
		invoke('npx-check', 'npx', ['--yes', `--package=${localUrl}`, 'demistify', 'check', ...pages]);
		invoke(
			'npx-observed-check',
			'npx',
			['--yes', `--package=${localUrl}`, 'demistify', 'check', ...pages],
			true,
		);
		const localArchive = join(work, 'trail-guide', 'demistify', archiveName);
		const relativePages = [
			'requirements.html',
			'specification.html',
			'program-design.html',
			'verification.html',
			'execution.html',
		].map((name): string => `./trail-guide/${name}`);
		const expectedLocalPrepare = `pnpm dlx "$PWD/trail-guide/demistify/${archiveName}" prepare ./trail-guide`;
		const expectedLocalCheck = `pnpm dlx "$PWD/trail-guide/demistify/${archiveName}" check ${relativePages.join(' ')}`;
		if (
			!examples.some((example) => example.line === expectedLocalPrepare) ||
			!examples.some((example) => example.line === expectedLocalCheck)
		)
			throw new Error('Staged local commands differ from the supported exact argument template.');
		invoke('pnpm-local-prepare', 'pnpm', ['dlx', localArchive, 'prepare', './trail-guide']);
		invoke('pnpm-local-check', 'pnpm', ['dlx', localArchive, 'check', ...relativePages]);
		invoke(
			'pnpm-local-observed-check',
			'pnpm',
			['dlx', localArchive, 'check', ...relativePages],
			true,
		);
		const tarPath = join(work, archiveName);
		await writeFile(tarPath, archive, { flag: 'wx' });
		const unpacked = spawnSync('tar', ['-xOf', tarPath, 'package/demistify.js'], {
			maxBuffer: 16 * 1024 * 1024,
		});
		if (unpacked.status !== 0 || !unpacked.stdout)
			throw new Error('Cannot read CLI entry from the declared archive.');
		const expectedEntry = createHash('sha256').update(unpacked.stdout).digest('hex');
		const identitySchema = z.object({
			pid: z.number(),
			inode: z.number(),
			size: z.number(),
			mtimeMs: z.number(),
			argv1: z.string(),
			realpath: z.string(),
			sha256: z.string(),
			name: z.string(),
			version: z.string(),
		});
		const identity = (await readFile(join(work, 'identity.jsonl'), 'utf8'))
			.trim()
			.split('\n')
			.map((line) => identitySchema.parse(JSON.parse(line)));
		if (
			identity.length !== 4 ||
			identity.some(
				(item) =>
					item.sha256 !== expectedEntry ||
					item.name !== '@sunstack/demistify-cli' ||
					item.version !== manifest.version,
			)
		)
			throw new Error(`Executable identity incomplete or differs from archive; inspect ${work}.`);
		const listing = spawnSync('tar', ['-tzf', tarPath], { encoding: 'utf8' });
		if (listing.status !== 0 || !listing.stdout)
			throw new Error('Cannot list archive package files.');
		const members = listing.stdout
			.trim()
			.split('\n')
			.filter((member): boolean => !member.endsWith('/'));
		if (
			!members.length ||
			members.some((member) => !member.startsWith('package/') || member.split('/').includes('..'))
		)
			throw new Error('Archive has invalid package paths.');
		const packageChecks = await Promise.all(
			members.map(async (member) => {
				const extracted = spawnSync('tar', ['-xOf', tarPath, member], {
					maxBuffer: 16 * 1024 * 1024,
				});
				if (extracted.status !== 0 || !extracted.stdout)
					throw new Error(`Cannot read archive member ${member}.`);
				const expected = createHash('sha256').update(extracted.stdout).digest('hex');
				const actual = await Promise.all(
					identity.map(async (item) =>
						createHash('sha256')
							.update(await readFile(join(dirname(item.realpath), member.slice('package/'.length))))
							.digest('hex'),
					),
				);
				return { member, expected, actual };
			}),
		);
		if (
			packageChecks.some((item) => item.actual.some((digestValue) => digestValue !== item.expected))
		)
			throw new Error('Selected package files differ from staged archive.');
		await writeFile(join(work, 'package-checks.json'), JSON.stringify(packageChecks, null, 2));
		const afterCache = await inventoryCache();
		await writeFile(join(work, 'cache-after.json'), JSON.stringify(afterCache, null, 2));
		const sameObservedExecutable =
			identity[0]?.realpath === identity[1]?.realpath &&
			identity[0]?.inode === identity[1]?.inode &&
			identity[0]?.mtimeMs === identity[1]?.mtimeMs;
		process.stdout.write(
			`${JSON.stringify({ expectedEntry, identity, sameObservedExecutable, cacheAfter: afterCache, firstCache: beforeCache.complete ? (beforeCache.matches.some((item) => item.spec === localUrl) ? 'present' : 'absent') : 'unknown', hostedRelease: loopback ? 'not-observed' : 'under-observation' }, null, 2)}\n`,
		);
		if (!sameObservedExecutable)
			throw new Error(
				'Observed repeated pnpm executable identity changed; cache reuse is unverified.',
			);
	}
}
