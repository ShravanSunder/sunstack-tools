import { execFile } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { lstat, open, readFile, rename, unlink, writeFile } from 'node:fs/promises';
import { basename, dirname, join } from 'node:path';
import { isDeepStrictEqual, promisify } from 'node:util';

const run = promisify(execFile);

async function refuseTrackedOutput(file: string): Promise<void> {
	const directory = dirname(file);
	for (let candidate = directory; ; candidate = dirname(candidate)) {
		// oxlint-disable-next-line no-await-in-loop -- search for the nearest Git checkout boundary.
		const git = await lstat(join(candidate, '.git')).catch((error: unknown) => {
			if (!missing(error)) throw error;
			return undefined;
		});
		if (git) break;
		if (candidate === dirname(candidate)) return;
	}
	const { stdout } = await run(
		'git',
		['--literal-pathspecs', 'ls-files', '-z', '--', basename(file)],
		{
			cwd: directory,
		},
	);
	if (stdout)
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

export async function planGeneratedJson(
	file: string,
	value: unknown,
	sameValue: (current: unknown, next: unknown) => boolean = isDeepStrictEqual,
): Promise<GeneratedFile | undefined> {
	await refuseTrackedOutput(file);
	const existing = await lstat(file).catch((error: unknown) => {
		if (!missing(error)) throw error;
		return undefined;
	});
	if (existing && !existing.isFile()) throw new Error(`Expected a generated file at ${file}.`);
	const previous = existing ? await readFile(file, 'utf8') : undefined;
	if (previous !== undefined) {
		try {
			if (sameValue(JSON.parse(previous), value)) return undefined;
		} catch {
			// A malformed previous output can be regenerated from validated inputs.
		}
	}
	return { file, previous, text: `${JSON.stringify(value, null, 2)}\n` };
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
