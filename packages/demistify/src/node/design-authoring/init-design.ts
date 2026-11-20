import { mkdir, readFile, unlink, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

import { sharedNavigationSchema } from '../../core/shared-navigation.ts';
import { subjectIcons } from '../../core/subject-icons.ts';

const designTypeSource = `// Authoring types for Demistify design.ts.
type SubjectIcon = ${subjectIcons.map((icon): string => JSON.stringify(icon)).join(' | ')};
type Target = { page: string; fragment?: string };
type Page = { id: string; title: string; href: string; icon: SubjectIcon };
type MapNode = {
  title: string;
  description: string;
  icon: SubjectIcon;
  target?: Target;
  children?: readonly MapNode[];
};
type Walkthrough = {
  id: string;
  title: string;
  description: string;
  steps: readonly { target: Target; narration: string; icon?: SubjectIcon }[];
};
type RecordFields = {
  id: string;
  title: string;
  destination: Required<Target>;
  sources: readonly { href: string; excerpt: string }[];
};
type Relationship<Relation extends string> = { relation: Relation; targetId: string };
type Authority =
  | { state: 'owner-decided'; owner: 'owner'; decision: string }
  | { state: 'owner-open'; owner: 'owner'; question: string }
  | { state: 'agent-rationale'; author: string; rationale: string; tradeoff: string };
type DesignRecord = RecordFields & (
  | { kind: 'need'; statement: string }
  | { kind: 'rule'; statement: string; relationships: readonly Relationship<'serves' | 'governed-by'>[] }
  | { kind: 'realization'; explanation: string; status: 'proposed'; relationships: readonly Relationship<'realizes'>[] }
  | { kind: 'planned-check'; status: 'planned'; relationships: readonly Relationship<'checks'>[];
      cases: readonly { given: string; expect: string }[] }
  | { kind: 'choice'; authority: Authority }
);
export type DesignCore = {
  pages: readonly Page[];
  mapGroups: readonly MapNode[];
  walkthroughs?: readonly Walkthrough[];
  records: readonly DesignRecord[];
};
`;

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
	const authorFile = join(setDirectory, 'design.ts');
	const typeFile = join(setDirectory, 'design-schema.d.ts');
	await writeFile(typeFile, designTypeSource, { flag: 'wx' });
	try {
		await writeFile(
			authorFile,
			`import type { DesignCore } from './design-schema.d.ts';\n\nexport default ${JSON.stringify(design, null, 2)} as const satisfies DesignCore;\n`,
			{ flag: 'wx' },
		);
	} catch (error) {
		await unlink(typeFile);
		throw error;
	}
	return `Created design.ts and design-schema.d.ts in ${setDirectory}. Add records and run demistify design check.`;
}
