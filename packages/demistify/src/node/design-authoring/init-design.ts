import { mkdir, readFile, realpath, unlink, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';

import { sharedNavigationSchema } from '../../core/shared-navigation.ts';
import { openDesignCheck } from './check-design.ts';
import { deliverySdkSource } from './delivery-sdk-source.ts';
import { commitDesignOutputs, planGeneratedText, refuseTrackedOutput } from './design-output.ts';
import { designTypeSource, designTypesFor } from './design-type-source.ts';
import {
	designIgnoreDirectory,
	generatedDesignPaths,
	planDesignIgnore,
} from './generated-ignore.ts';
import { deliveryEntryFilename, deliverySdkFilename } from './read-delivery.ts';
import { designDeclarationFilename, readDesignItems, readDesignSources } from './read-design.ts';
import { findRegisteredProject } from './read-project.ts';

const authoringSupport = [
	[designDeclarationFilename, designTypeSource],
	[deliverySdkFilename, deliverySdkSource],
] as const;

const decisionsSource = `import type { DesignDecision } from './${designDeclarationFilename}';

export default [] as const satisfies readonly DesignDecision[];
`;
const assumptionsSource = `import type { DesignAssumption } from './${designDeclarationFilename}';

export default [] as const satisfies readonly DesignAssumption[];
`;
const deliverySource = `import design from './design.ts';
import { defineDelivery } from './${deliverySdkFilename}';

export default defineDelivery(design, []);
`;

async function authoredRecordIds(directory: string): Promise<string[]> {
	const project = await findRegisteredProject(directory);
	const check = await openDesignCheck(project?.directory ?? directory);
	async function readRecords(setDirectory: string): Promise<string[]> {
		const read = (file: string): Promise<string> => check.read(file, setDirectory);
		const { design } = await readDesignSources(setDirectory, read, async (file) =>
			read(file).catch((error: unknown): undefined => {
				if (error instanceof Error && 'code' in error && error.code === 'ENOENT') return undefined;
				throw error;
			}),
		);
		return design.records.map(({ id }) => id);
	}
	const localIds = await readRecords(directory);
	if (!project) return localIds;

	const { directory: projectDirectory, manifest } = project;
	const sets = Object.entries(manifest.designs);
	const milestoneIds = await Promise.all(
		sets.map(async ([name, path]): Promise<string[]> => {
			const setDirectory = await realpath(dirname(path));
			const ids = setDirectory === directory ? localIds : await readRecords(setDirectory);
			return ids.map((id): string => `${name}/${id}`);
		}),
	);
	const shared = await Promise.all(
		[
			{ path: manifest.decisions, kind: 'choice' as const },
			{ path: manifest.assumptions, kind: 'assumption' as const },
		].map(async ({ path, kind }): Promise<string[]> =>
			path === undefined
				? []
				: readDesignItems(
						path,
						await check.read(path, projectDirectory),
						kind,
						projectDirectory,
					).map(({ id }) => `project/${id}`),
		),
	);
	return [...localIds, ...milestoneIds.flat(), ...shared.flat()];
}

/** Refresh editor support for an existing authored design. */
export async function refreshDesignTypes(directory: string): Promise<string> {
	const setDirectory = await realpath(resolve(directory));
	const ids = await authoredRecordIds(setDirectory);
	const ignoreDirectory = await designIgnoreDirectory(setDirectory);
	await commitDesignOutputs(
		[setDirectory, ignoreDirectory],
		[
			{
				verify: async () => {
					if (JSON.stringify(await authoredRecordIds(setDirectory)) !== JSON.stringify(ids))
						throw new Error('Design record IDs changed while refreshing types.');
				},
				plan: () =>
					Promise.all([
						planDesignIgnore(ignoreDirectory, generatedDesignPaths([setDirectory])),
						...authoringSupport.map(([name, source]) =>
							planGeneratedText(
								join(setDirectory, name),
								name === designDeclarationFilename ? designTypesFor(ids) : source,
							),
						),
					]),
			},
		],
	);
	return `Refreshed ${designDeclarationFilename} and ${deliverySdkFilename} in ${setDirectory}.`;
}

/** Start from the current Pages, Map and Walkthroughs when a sidecar already exists. */
export async function initDesignSet(directory: string): Promise<string> {
	const requested = resolve(directory);
	await mkdir(requested, { recursive: true });
	const setDirectory = await realpath(requested);
	const ignoreDirectory = await designIgnoreDirectory(setDirectory);
	const planIgnore = (): ReturnType<typeof planDesignIgnore> =>
		planDesignIgnore(ignoreDirectory, generatedDesignPaths([setDirectory]));
	await Promise.all([
		planIgnore(),
		...authoringSupport.map(([name]) => refuseTrackedOutput(join(setDirectory, name))),
	]);
	const existing = await readFile(join(setDirectory, 'navigation.json'), 'utf8').catch(
		(error: unknown): undefined => {
			if (error instanceof Error && 'code' in error && error.code === 'ENOENT') return undefined;
			throw error;
		},
	);
	const navigation =
		existing === undefined ? undefined : sharedNavigationSchema.parse(JSON.parse(existing));
	const design = {
		pages: navigation?.pages ?? [{ id: 'home', title: 'Home', href: 'index.html', icon: 'file' }],
		mapGroups: navigation?.map ?? [],
		...(navigation?.walkthroughs.length ? { walkthroughs: navigation.walkthroughs } : {}),
		records: [],
	};
	const created: string[] = [];
	try {
		const designSource = `import { defineDesign } from './${deliverySdkFilename}';\n\nexport default defineDesign(${JSON.stringify(design, null, 2)});\n`;
		const initialFiles = [
			...authoringSupport,
			['design.ts', designSource],
			['design-decisions.ts', decisionsSource],
			['design-assumptions.ts', assumptionsSource],
			[deliveryEntryFilename, deliverySource],
		] as const;
		for (const [name, text] of initialFiles) {
			const file = join(setDirectory, name);
			// oxlint-disable-next-line no-await-in-loop -- roll back only the files this invocation created.
			await writeFile(file, text, { flag: 'wx' });
			created.push(file);
		}
		await commitDesignOutputs(
			[setDirectory, ignoreDirectory],
			[
				{
					verify: (): Promise<void> => Promise.resolve(),
					plan: async () => [await planIgnore()],
				},
			],
		);
	} catch (error) {
		await Promise.all(created.map((file) => unlink(file)));
		throw error;
	}
	return `Created design.ts, design-decisions.ts, design-assumptions.ts, ${deliveryEntryFilename} and generated SDK support in ${setDirectory}. Supply the declared HTML page (index.html by default), then run demistify design check.`;
}
