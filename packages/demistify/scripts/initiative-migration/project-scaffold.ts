import { join } from 'node:path';

import type { AddedRow } from './recut-resolution.ts';
import { sdkFilename, type RecutProject, type SourceWorkspace } from './recut-workspace.ts';
import { isIdentifierName } from './source-edits.ts';

/** Files the re-cut writes for a project it creates, and for an added later row. */
export const designTemplate = (
	title: string,
): string => `import { defineDesign } from './${sdkFilename}';

export default defineDesign({
  pages: [{ id: 'home', title: ${JSON.stringify(title)}, href: 'index.html', icon: 'file' }],
  mapGroups: [],
  records: [],
});
`;
export const recordsTemplate = 'export default [];\n';
export const deliveryTemplate = (
	lead: string | undefined,
): string => `import design from './design.ts';
import { defineDelivery } from './${sdkFilename}';

export default defineDelivery(design, []${lead ? `, { lead: ${JSON.stringify(lead)} }` : ''});
`;

export function scaffoldProjectSources(
	project: RecutProject,
	title: string,
	lead: string | undefined,
	workspace: SourceWorkspace,
): void {
	workspace.write(project.designFile, designTemplate(title));
	workspace.write(join(project.directory, 'design-decisions.ts'), recordsTemplate);
	workspace.write(join(project.directory, 'design-assumptions.ts'), recordsTemplate);
	workspace.write(join(project.directory, 'design-delivery.ts'), deliveryTemplate(lead));
}

export function laterDeliverableSource(row: AddedRow): string {
	return `import { defineDeliverable } from '../${sdkFilename}';

export default defineDeliverable({
  kind: 'later',
  id: ${JSON.stringify(row.id)},
  title: ${JSON.stringify(row.title)},
});
`;
}

export function slug(text: string, words = 6): string {
	return text
		.toLowerCase()
		.replace(/[^a-z0-9]+/gu, ' ')
		.trim()
		.split(' ')
		.filter(Boolean)
		.slice(0, words)
		.join('-');
}

export function camelBinding(text: string, taken: ReadonlySet<string>): string {
	const words = slug(text, 5).split('-').filter(Boolean);
	const [first = 'deliverable', ...rest] = words;
	let base = `${first}${rest.map((word) => word.charAt(0).toUpperCase() + word.slice(1)).join('')}`;
	if (!isIdentifierName(base)) base = `deliverable${base.charAt(0).toUpperCase()}${base.slice(1)}`;
	let candidate = base;
	for (let counter = 2; taken.has(candidate); counter += 1) candidate = `${base}${counter}`;
	return candidate;
}
