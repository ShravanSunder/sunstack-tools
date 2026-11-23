import { execFile } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { lstat, open, readFile, rename, unlink, writeFile } from 'node:fs/promises';
import { basename, dirname, join } from 'node:path';
import { isDeepStrictEqual, promisify } from 'node:util';

const run = promisify(execFile);

export async function gitOutputRoot(directory: string): Promise<string | undefined> {
	for (let candidate = directory; ; candidate = dirname(candidate)) {
		// oxlint-disable-next-line no-await-in-loop -- search for the nearest Git checkout boundary.
		const git = await lstat(join(candidate, '.git')).catch((error: unknown) => {
			if (!missing(error)) throw error;
			return undefined;
		});
		if (git) return candidate;
		if (candidate === dirname(candidate)) return undefined;
	}
}

export async function isTrackedOutput(file: string): Promise<boolean> {
	const directory = dirname(file);
	if (!(await gitOutputRoot(directory))) return false;
	const { stdout } = await run(
		'git',
		['--literal-pathspecs', 'ls-files', '-z', '--', basename(file)],
		{
			cwd: directory,
		},
	);
	return stdout.length > 0;
}

export async function refuseTrackedOutput(file: string): Promise<void> {
	if (await isTrackedOutput(file))
		throw new Error(
			`Generated output is tracked: ${file}. Remove it from the Git index before using --write.`,
		);
}

function missing(error: unknown): boolean {
	return error instanceof Error && 'code' in error && error.code === 'ENOENT';
}

export interface GeneratedFile {
	file: string;
	previous: string | undefined;
	text: string;
}

export async function readOutputFile(
	file: string,
	description = 'generated file',
): Promise<string | undefined> {
	const existing = await lstat(file).catch((error: unknown) => {
		if (!missing(error)) throw error;
		return undefined;
	});
	if (existing && !existing.isFile()) throw new Error(`Expected a ${description} at ${file}.`);
	return existing ? readFile(file, 'utf8') : undefined;
}

export async function planGeneratedText(
	file: string,
	text: string,
	options: { readonly omitIfAbsent?: boolean } = {},
): Promise<GeneratedFile | undefined> {
	await refuseTrackedOutput(file);
	const previous = await readOutputFile(file);
	if ((previous === undefined && options.omitIfAbsent) || previous === text) return undefined;
	return { file, previous, text };
}

export async function planGeneratedJson(
	file: string,
	value: unknown,
): Promise<GeneratedFile | undefined> {
	await refuseTrackedOutput(file);
	const previous = await readOutputFile(file);
	const text = `${JSON.stringify(value, null, 2)}\n`;
	if (previous !== undefined) {
		try {
			if (isDeepStrictEqual(JSON.parse(previous), JSON.parse(text))) return undefined;
		} catch {
			// A malformed previous output can be regenerated from validated inputs.
		}
	}
	return { file, previous, text };
}

/** Preflight all outputs, then stage their replacements and rollback copies together. */
async function writeGeneratedFiles(outputs: GeneratedFile[]): Promise<void> {
	const writes = outputs.map((output) => ({
		...output,
		stage: `${output.file}.${randomUUID()}.tmp`,
		rollback: `${output.file}.${randomUUID()}.tmp`,
	}));
	const committed: typeof writes = [];
	try {
		for (const { stage, text, previous, rollback } of writes) {
			// oxlint-disable-next-line no-await-in-loop -- finish staging before renaming either output.
			await writeFile(stage, text, { flag: 'wx' });
			if (previous !== undefined) {
				// oxlint-disable-next-line no-await-in-loop -- stage restoration bytes before renaming.
				await writeFile(rollback, previous, { flag: 'wx' });
			}
		}
		for (const pending of writes) {
			// oxlint-disable-next-line no-await-in-loop -- record each completed rename for rollback.
			await rename(pending.stage, pending.file);
			committed.push(pending);
		}
	} catch (error) {
		for (const { file, previous, rollback } of committed.toReversed()) {
			// oxlint-disable-next-line no-await-in-loop -- undo each completed write before reporting failure.
			await (previous === undefined ? unlink(file) : rename(rollback, file));
		}
		throw error;
	} finally {
		await Promise.all(
			writes
				.flatMap(({ stage, rollback }) => [stage, rollback])
				.map(async (path) =>
					unlink(path).catch((error: unknown): void => {
						if (!missing(error)) throw error;
					}),
				),
		);
	}
}

export interface OutputCheck {
	readonly verify: () => Promise<void>;
	readonly plan: () => Promise<readonly (GeneratedFile | undefined)[]>;
}

/** Lock every output directory before checking inputs and committing the complete batch. */
export async function commitDesignOutputs(
	directories: readonly string[],
	checks: readonly OutputCheck[],
): Promise<void> {
	const locks: { path: string; handle: Awaited<ReturnType<typeof open>> }[] = [];
	try {
		for (const directory of [...new Set(directories)].toSorted()) {
			const path = join(directory, '.demistify-design.lock');
			// oxlint-disable-next-line no-await-in-loop -- retain acquired handles to release them if a later lock fails.
			const handle = await open(path, 'wx').catch((error: unknown) => {
				if (error instanceof Error && 'code' in error && error.code === 'EEXIST')
					throw new Error(`Another design check has the output lock: ${path}`, { cause: error });
				throw error;
			});
			locks.push({ path, handle });
		}
		await Promise.all(checks.map(({ verify }) => verify()));
		const outputs = (await Promise.all(checks.map(({ plan }) => plan()))).flat();
		await writeGeneratedFiles(
			outputs.filter((output): output is GeneratedFile => output !== undefined),
		);
	} finally {
		await Promise.all(
			locks.map(async ({ path, handle }) => {
				await handle.close();
				await unlink(path);
			}),
		);
	}
}
