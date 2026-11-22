import { randomUUID } from 'node:crypto';
import { mkdir, readFile, rename, unlink, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

import { sharedNavigationSchema } from '../../core/shared-navigation.ts';
import { subjectIcons } from '../../core/subject-icons.ts';
import { deliverySdkSource } from './delivery-sdk-source.ts';
import { deliveryEntryFilename, deliverySdkFilename } from './read-delivery.ts';
import { designDeclarationFilename } from './read-design.ts';

const designTypeSource = `// Authoring types for Demistify design, decision and assumption files.
type SubjectIcon = ${subjectIcons.map((icon): string => JSON.stringify(icon)).join(' | ')};
type Target = { page: string; fragment?: string };
type NavigationTarget = Target | { record: string };
type Page = { id: string; title: string; href: string; icon: SubjectIcon };
type MapNode = {
  title: string;
  description: string;
  icon: SubjectIcon;
  target?: NavigationTarget;
  children?: readonly MapNode[];
};
type Walkthrough = {
  id: string;
  title: string;
  description: string;
  steps: readonly { target: NavigationTarget; narration: string; icon?: SubjectIcon }[];
};
type SourceReference = { href: string; excerpt: string; verbatim?: true };
type RecordFields = {
  id: string;
  title: string;
  destination: Required<Target>;
  sources: readonly SourceReference[];
};
type Relationship<Relation extends string> = { relation: Relation; targetId: string };
type DesignDependency = 'governed-by' | 'relies-on';
type SettledDecision = { decision: string; decidedOn: string };
type Authority =
  | ({ state: 'owner-decided'; owner: 'owner'; sourceKind: 'owner-words' | 'owner-accepted' } & SettledDecision)
  | { state: 'owner-open'; owner: 'owner'; question: string }
  | ({ state: 'lead-decided'; lead: string; sourceKind: 'author' | 'contract'; rationale: string } & SettledDecision);
export type DesignRecord = RecordFields & (
  | { kind: 'need'; statement: string }
  | { kind: 'rule'; statement: string; relationships: readonly Relationship<'serves' | DesignDependency>[] }
  | { kind: 'realization'; explanation: string; status: 'proposed'; relationships: readonly Relationship<'realizes' | DesignDependency>[] }
  | { kind: 'planned-check'; status: 'planned'; relationships: readonly Relationship<'checks' | DesignDependency>[];
      cases: readonly { given: string; expect: string }[] }
);
export type DesignDecision = RecordFields & { kind: 'choice'; authority: Authority; supersedes?: string };
export type DesignAssumption = RecordFields & { kind: 'assumption'; statement: string } & (
  { status: 'open'; resolution?: never } | { status: 'confirmed' | 'refuted'; resolution: SourceReference }
);
export type DesignCore = {
  pages: readonly Page[];
  mapGroups: readonly MapNode[];
  walkthroughs?: readonly Walkthrough[];
  records: readonly DesignRecord[];
  acceptances?: readonly { deliverableId: string; build: string; scopeRevision: string; evidence: string }[];
};
`;

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

/** Refresh editor support for an existing authored design. */
export async function refreshDesignTypes(directory: string): Promise<string> {
	const setDirectory = resolve(directory);
	await readFile(join(setDirectory, 'design.ts'));
	await Promise.all(
		authoringSupport.map(async ([name, text]) => {
			const file = join(setDirectory, name);
			const temporary = `${file}.${randomUUID()}.tmp`;
			await writeFile(temporary, text, { flag: 'wx' });
			try {
				await rename(temporary, file);
			} catch (error) {
				await unlink(temporary);
				throw error;
			}
		}),
	);
	return `Refreshed ${designDeclarationFilename} and ${deliverySdkFilename} in ${setDirectory}.`;
}

/** Start from the current Pages, Map and Walkthroughs when a sidecar already exists. */
export async function initDesignSet(directory: string): Promise<string> {
	const setDirectory = resolve(directory);
	await mkdir(setDirectory, { recursive: true });
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
		const designSource = `import type { DesignCore } from './${designDeclarationFilename}';\n\nexport default ${JSON.stringify(design, null, 2)} as const satisfies DesignCore;\n`;
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
	} catch (error) {
		await Promise.all(created.map((file) => unlink(file)));
		throw error;
	}
	return `Created design.ts, design-decisions.ts, design-assumptions.ts, ${deliveryEntryFilename} and generated SDK support in ${setDirectory}. Add records and run demistify design check.`;
}
