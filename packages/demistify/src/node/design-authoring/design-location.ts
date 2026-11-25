import { stat } from 'node:fs/promises';
import { dirname, extname, isAbsolute, join, relative, resolve } from 'node:path';

import { containsPath } from '../local-site.ts';

export async function findDesignInput(
	directory: string,
	name: 'design.ts' | 'initiative.ts',
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

export function resolveDesignLink(
	href: string,
	from: string,
	siteRoot: string,
): { file: string; fragment: string } | undefined {
	if (/^[a-z][a-z0-9+.-]*:|^\/\//i.test(href)) return undefined;
	const hash = href.indexOf('#');
	const address = hash < 0 ? href : href.slice(0, hash);
	const pathname = decodeURIComponent(address.split('?')[0] ?? '');
	const fragment = hash < 0 ? '' : decodeURIComponent(href.slice(hash + 1));
	const file = pathname ? resolve(dirname(from), pathname) : from;
	if (isAbsolute(pathname) || pathname.includes('\\') || !containsPath(siteRoot, file))
		throw new Error(
			`Relative link escapes the site root: ${href} from ${relative(siteRoot, from)}`,
		);
	return { file, fragment };
}

export function declaredDesignPagePath(href: string, from: string, siteRoot: string): string {
	const target = resolveDesignLink(href, from, siteRoot);
	if (!target || /[?#]/.test(href) || !/^\.html?$/i.test(extname(target.file)))
		throw new Error(`A declared page must name a relative HTML file: ${href}`);
	return target.file;
}
