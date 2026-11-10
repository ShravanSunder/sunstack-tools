import { dirname, join } from 'node:path';

import {
	sharedNavigationSchema,
	type NavigationTarget,
	type SharedMapNode,
} from '../core/shared-navigation.ts';

interface NavigationProblem {
	readonly severity: 'error' | 'warning';
	readonly message: string;
}

/** Read the same shared contract as the browser, including its actual local destinations. */
export async function checkSharedNavigation(props: {
	readonly reference: string;
	readonly readSource: ((path: string) => Promise<string | undefined>) | undefined;
	readonly idsOnPage: (path: string) => Promise<ReadonlySet<string> | undefined>;
}): Promise<readonly NavigationProblem[]> {
	const { reference, readSource, idsOnPage } = props;
	if (/^[a-z][a-z0-9+.-]*:|^\/\//i.test(reference))
		return [
			{
				severity: 'warning',
				message:
					'Remote shared navigation is validated at runtime; this check reads local sources only.',
			},
		];
	const source = await readSource?.(reference);
	if (source === undefined)
		return [{ severity: 'error', message: `Missing shared navigation source: ${reference}` }];
	let raw: unknown;
	try {
		raw = JSON.parse(source);
	} catch {
		return [{ severity: 'error', message: `Malformed shared navigation JSON: ${reference}` }];
	}
	const parsed = sharedNavigationSchema.safeParse(raw);
	if (!parsed.success)
		return parsed.error.issues.map((issue): NavigationProblem => ({
			severity: 'error',
			message: issue.message,
		}));
	const navigation = parsed.data;
	const pages = await Promise.all(
		navigation.pages.map(async (page) => {
			if (/^[a-z][a-z0-9+.-]*:|^\/\//i.test(page.href))
				return { page, file: undefined, ids: undefined };
			const pathname = decodeURIComponent(page.href.split(/[?#]/)[0] ?? '');
			const file = join(dirname(reference), pathname);
			return { page, file, ids: await idsOnPage(file) };
		}),
	);
	const problems: NavigationProblem[] = [];
	for (const page of pages)
		if (page.file && !page.ids)
			problems.push({ severity: 'error', message: `Missing shared page: ${page.file}` });
	const targets: NavigationTarget[] = [];
	const visit = (node: SharedMapNode): void => {
		if (node.target) targets.push(node.target);
		for (const child of node.children) visit(child);
	};
	for (const node of navigation.map) visit(node);
	for (const route of navigation.walkthroughs)
		for (const step of route.steps) targets.push(step.target);
	const checked = new Set<string>();
	for (const target of targets) {
		const key = `${target.page}#${target.fragment ?? ''}`;
		if (checked.has(key)) continue;
		checked.add(key);
		const page = pages.find((entry): boolean => entry.page.id === target.page);
		if (page?.ids && target.fragment && !page.ids.has(target.fragment))
			problems.push({
				severity: 'error',
				message: `Missing shared navigation target: ${page.file}#${target.fragment}`,
			});
	}
	return problems;
}
