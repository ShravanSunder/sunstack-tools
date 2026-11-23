import { stat } from 'node:fs/promises';
import { dirname, join } from 'node:path';

export async function findDesignInput(
	directory: string,
	name: 'design.ts' | 'project.ts',
	limit?: string,
): Promise<string | undefined> {
	for (let candidate = directory; ; candidate = dirname(candidate)) {
		const file = join(candidate, name);
		// oxlint-disable-next-line no-await-in-loop -- the nearest input wins.
		const exists = await stat(file).then(
			(info): boolean => info.isFile(),
			(error: unknown): boolean => {
				if (error instanceof Error && 'code' in error && error.code === 'ENOENT') return false;
				throw error;
			},
		);
		if (exists) return candidate;
		if (candidate === limit || candidate === dirname(candidate)) return undefined;
	}
}
