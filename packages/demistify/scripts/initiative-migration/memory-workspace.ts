import { readFileSync } from 'node:fs';
import { mkdir, unlink, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import type { FileChange } from './migration-report.ts';
import type { SourceWorkspace } from './migration-workspace.ts';

function missing(error: unknown): boolean {
	return (
		error instanceof Error && 'code' in error && ['ENOENT', 'ENOTDIR'].includes(String(error.code))
	);
}

/**
 * Every change the run makes, held in memory until it is applied. Paths name the
 * working tree (the initiative itself, or a converted copy of a project.ts tree);
 * originals always come from the initiative as it is on disk.
 */
export class MemoryWorkspace implements SourceWorkspace {
	readonly changes = new Map<string, string | null>();
	readonly #originals = new Map<string, string | undefined>();
	readonly #originRoot: string;
	readonly #workRoot: string;

	constructor(originRoot: string, workRoot: string) {
		this.#originRoot = originRoot;
		this.#workRoot = workRoot;
	}

	original(file: string): string | undefined {
		if (this.#originals.has(file)) return this.#originals.get(file);
		let text: string | undefined;
		try {
			text = readFileSync(join(this.#originRoot, relative(this.#workRoot, file)), 'utf8');
		} catch (error) {
			if (!missing(error)) throw error;
		}
		this.#originals.set(file, text);
		return text;
	}

	read(file: string): string | undefined {
		if (this.changes.has(file)) return this.changes.get(file) ?? undefined;
		return this.original(file);
	}

	write(file: string, text: string): void {
		if (this.original(file) === text) this.changes.delete(file);
		else this.changes.set(file, text);
	}

	remove(file: string): void {
		if (this.original(file) === undefined) this.changes.delete(file);
		else this.changes.set(file, null);
	}

	fileChanges(): FileChange[] {
		return [...this.changes]
			.map(([file, text]): FileChange => ({
				path: relative(this.#workRoot, file),
				kind: text === null ? 'removed' : this.original(file) === undefined ? 'created' : 'changed',
			}))
			.toSorted((left, right) => left.path.localeCompare(right.path));
	}

	/** Write the change set into a tree with the same layout as the working tree. */
	async applyTo(target: string): Promise<void> {
		for (const [file, text] of this.changes) {
			const destination = join(target, relative(this.#workRoot, file));
			if (text === null) {
				// oxlint-disable-next-line no-await-in-loop -- apply the change set in a stable order.
				await unlink(destination).catch((error: unknown) => {
					if (!missing(error)) throw error;
				});
				continue;
			}
			// oxlint-disable-next-line no-await-in-loop -- apply the change set in a stable order.
			await mkdir(dirname(destination), { recursive: true });
			// oxlint-disable-next-line no-await-in-loop -- apply the change set in a stable order.
			await writeFile(destination, text);
		}
	}
}
