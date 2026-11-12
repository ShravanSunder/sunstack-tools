import { spawnSync } from 'node:child_process';
/** Pin the fresh installed consumer and its native bytes before any SDK call. */
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

import { z } from 'zod';

const digestSchema = z.string().regex(/^[a-f0-9]{64}$/);
const absolutePathSchema = z.string().refine(path.isAbsolute, 'Use an absolute path');
export const sdkTargetSchema = z.strictObject({
	sourceRoot: absolutePathSchema,
	commit: z.string().regex(/^[a-f0-9]{40}$/),
	consumerRoot: absolutePathSchema,
	packageRoot: absolutePathSchema,
	entry: absolutePathSchema,
	addon: absolutePathSchema,
	tarball: absolutePathSchema,
	tarballSha256: digestSchema,
	files: z.array(z.strictObject({ path: absolutePathSchema, sha256: digestSchema })).min(3),
	release: z.string().min(1),
	contractRevision: z.number().int().nonnegative(),
	platform: z.enum(['darwin', 'linux']),
	arch: z.enum(['arm64', 'x64']),
	libc: z.enum(['none', 'glibc', 'musl']),
	cli: z.strictObject({ binary: absolutePathSchema, sha256: digestSchema }).optional(),
});
export type SdkTarget = z.infer<typeof sdkTargetSchema>;
const callSchema = z.custom<(...arguments_: unknown[]) => Promise<unknown>>(
	(value) => typeof value === 'function',
	'Installed SDK must export a callable entry',
);
const sdkExportsSchema = z.object({
	load: callSchema,
	check: callSchema,
	run: callSchema,
	contractInfo: callSchema,
});
export type SdkExports = z.infer<typeof sdkExportsSchema>;

export function fileDigest(file: string): string {
	return createHash('sha256').update(filesystem.readFileSync(file)).digest('hex');
}
export function confirmSdkFilePins(files: readonly { path: string; sha256: string }[]): void {
	const unique = new Set(files.map((file) => file.path));
	requireSdk(
		unique.size === files.length && unique.size > 0,
		'Duplicate or empty pinned artifact list',
	);
	for (const file of files) {
		requireSdk(filesystem.realpathSync(file.path) === file.path, 'Pinned file uses a link');
		requireSdk(
			fileDigest(file.path) === file.sha256,
			`Installed artifact digest differs: ${file.path}`,
		);
	}
}
export function requireSdk(condition: unknown, message: string): asserts condition {
	if (!condition) throw new Error(message);
}
export function containedPath(root: string, file: string): boolean {
	const relative = path.relative(root, file);
	return (
		relative === '' ||
		(!relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative))
	);
}
/** Require the complete wrapper and selected native-package payload, not selected entry files. */
export function confirmSdkPayloadPins(options: {
	consumerRoot: string;
	packageRoot: string;
	addon: string;
	files: readonly { path: string; sha256: string }[];
}): void {
	let nativeRoot = path.dirname(options.addon);
	while (!filesystem.existsSync(path.join(nativeRoot, 'package.json'))) {
		const parent = path.dirname(nativeRoot);
		requireSdk(
			parent !== nativeRoot && containedPath(options.consumerRoot, parent),
			'Native artifact has no contained package manifest',
		);
		nativeRoot = parent;
	}
	requireSdk(
		nativeRoot !== options.consumerRoot,
		'Native artifact must belong to its installed package',
	);
	const pinned = new Set(options.files.map((file) => file.path));
	const visit = (directory: string): void => {
		for (const entry of filesystem.readdirSync(directory, { withFileTypes: true })) {
			const file = path.join(directory, entry.name);
			requireSdk(
				!entry.isSymbolicLink() && filesystem.realpathSync(file) === file,
				'Installed SDK payload contains a link',
			);
			if (entry.isDirectory()) visit(file);
			else
				requireSdk(entry.isFile() && pinned.has(file), `SDK payload file is not pinned: ${file}`);
		}
	};
	for (const root of new Set([options.packageRoot, nativeRoot])) {
		requireSdk(
			containedPath(options.consumerRoot, root) && filesystem.realpathSync(root) === root,
			'Payload package escapes canonical consumer',
		);
		visit(root);
	}
	confirmSdkFilePins(options.files);
}
export function confirmSdkTarget(target: SdkTarget): void {
	requireSdk(Number(process.versions.node.split('.')[0]) === 26, 'SDK journey requires Node 26');
	requireSdk(
		process.platform === target.platform && process.arch === target.arch,
		'Wrong execution platform/architecture',
	);
	requireSdk((target.platform === 'darwin') === (target.libc === 'none'), 'Invalid libc cell');
	if (target.platform === 'linux') {
		const { header } = z
			.object({ header: z.object({ glibcVersionRuntime: z.string().optional() }) })
			.parse(process.report.getReport());
		requireSdk(
			(header.glibcVersionRuntime ? 'glibc' : 'musl') === target.libc,
			'Wrong runtime libc',
		);
	}
	const head = spawnSync('git', ['rev-parse', 'HEAD'], {
		cwd: target.sourceRoot,
		encoding: 'utf8',
	});
	const dirty = spawnSync(
		'git',
		[
			'status',
			'--porcelain',
			'--',
			'crates',
			'packages/sdk',
			'Cargo.toml',
			'Cargo.lock',
			'package.json',
			'pnpm-lock.yaml',
			'pnpm-workspace.yaml',
			'rust-toolchain.toml',
			'.cargo',
		],
		{ cwd: target.sourceRoot, encoding: 'utf8' },
	);
	requireSdk(
		head.status === 0 &&
			head.stdout.trim() === target.commit &&
			dirty.status === 0 &&
			dirty.stdout === '',
		'Product source moved or is dirty',
	);
	for (const file of [target.consumerRoot, target.packageRoot, target.entry, target.addon]) {
		requireSdk(
			filesystem.realpathSync(file) === file,
			'Consumer/package/entry/addon must use canonical paths, without workspace links',
		);
		requireSdk(
			!containedPath(target.sourceRoot, file),
			'Fresh consumer and installed artifacts must be outside source checkout',
		);
	}
	requireSdk(
		containedPath(target.consumerRoot, target.packageRoot),
		'SDK package is outside fresh consumer',
	);
	requireSdk(
		containedPath(target.packageRoot, target.entry),
		'SDK entry is outside installed package',
	);
	requireSdk(
		containedPath(target.consumerRoot, target.addon) && target.addon.endsWith('.node'),
		'Addon is not an installed native artifact',
	);
	const manifestPath = path.join(target.packageRoot, 'package.json');
	const manifest = z
		.object({
			name: z.literal('@sunstack/sdk'),
			version: z.string(),
			dependencies: z.record(z.string(), z.string()).optional(),
			exports: z.unknown().optional(),
		})
		.parse(JSON.parse(filesystem.readFileSync(manifestPath, 'utf8')));
	requireSdk(manifest.version === target.release, 'Installed SDK release differs');
	requireSdk(
		!manifest.dependencies?.['zod'] &&
			!(JSON.stringify(manifest.exports) ?? '').toLowerCase().includes('zod'),
		'Base SDK advertises/depends on Zod',
	);
	const resolver = createRequire(path.join(target.consumerRoot, 'package.json'));
	requireSdk(
		resolver.resolve('@sunstack/sdk') === target.entry,
		'Consumer resolves another SDK entry',
	);
	let hasZod = false;
	try {
		resolver.resolve('zod');
		hasZod = true;
	} catch {
		/* The fresh consumer must not resolve Zod. */
	}
	requireSdk(!hasZod, 'Fresh consumer has ambient Zod');
	const files = new Map(target.files.map((file) => [file.path, file.sha256]));
	requireSdk(files.size === target.files.length, 'Duplicate pinned artifact path');
	for (const required of [target.entry, target.addon, manifestPath])
		requireSdk(files.has(required), 'Target does not pin entry/addon/package manifest');
	for (const file of target.files) {
		requireSdk(
			containedPath(target.consumerRoot, file.path),
			'Pinned installed file escapes consumer',
		);
	}
	confirmSdkPayloadPins(target);
	requireSdk(
		fileDigest(target.tarball) === target.tarballSha256,
		'Packed candidate digest differs',
	);
	if (target.cli)
		requireSdk(fileDigest(target.cli.binary) === target.cli.sha256, 'CLI candidate digest differs');
}
export async function importInstalledSdk(target: SdkTarget): Promise<SdkExports> {
	confirmSdkTarget(target);
	const cell = `${target.platform}-${target.arch}${target.libc === 'none' ? '' : `-${target.libc === 'glibc' ? 'gnu' : 'musl'}`}`;
	requireSdk(
		createRequire(target.entry).resolve(`@sunstack/sdk-${cell}`) === target.addon,
		'Installed SDK resolves another native addon',
	);
	const imported: unknown = await import(pathToFileURL(target.entry).href);
	return sdkExportsSchema.parse(imported);
}

export async function inspectInstalledSdkContract(
	sdk: Pick<SdkExports, 'contractInfo'>,
	target: Pick<SdkTarget, 'release' | 'contractRevision' | 'arch' | 'platform' | 'libc'>,
): Promise<unknown> {
	const reply = await sdk.contractInfo();
	const ready = z
		.object({
			kind: z.literal('ready'),
			contract: z.object({
				sdk_release: z.string(),
				contract_revision: z.number().int(),
				target: z.string(),
			}),
		})
		.parse(reply);
	const cpu = target.arch === 'arm64' ? 'aarch64' : 'x86_64';
	const expectedTarget = `${cpu}-${target.platform === 'darwin' ? 'apple-darwin' : `unknown-linux-${target.libc === 'glibc' ? 'gnu' : 'musl'}`}`;
	requireSdk(
		ready.contract.sdk_release === target.release &&
			ready.contract.contract_revision === target.contractRevision &&
			ready.contract.target === expectedTarget,
		'Installed native ContractInfo differs from declared SDK target',
	);
	return reply;
}

export function saveSdkEvidence(file: string, value: unknown): void {
	filesystem.writeFileSync(
		file,
		`${JSON.stringify(value, (_key, item: unknown) => (typeof item === 'bigint' ? { bigint_decimal: String(item) } : item), 2)}\n`,
		{ flag: 'wx' },
	);
}
