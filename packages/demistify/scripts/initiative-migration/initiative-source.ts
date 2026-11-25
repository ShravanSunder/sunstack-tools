import { relative, sep } from 'node:path';

import { isIdentifierName } from './source-edits.ts';

export interface InitiativeProjectEntry {
	readonly key: string;
	/** Absolute path of the project's design.ts (or design-delivery.ts) import. */
	readonly design: string;
	readonly number: number;
	readonly achieves: string | undefined;
	readonly acceptance: string | undefined;
}

interface InitiativeSourceInput {
	readonly root: string;
	readonly projects: readonly InitiativeProjectEntry[];
	readonly decisions: string | undefined;
	readonly assumptions: string | undefined;
	readonly identities: string | undefined;
}

function specifier(root: string, file: string): string {
	return `./${relative(root, file).split(sep).join('/')}`;
}

function bindingFor(key: string, taken: Set<string>): string {
	const [first = 'project', ...rest] = key.split('-').filter(Boolean);
	let base = `${first}${rest.map((word) => word.charAt(0).toUpperCase() + word.slice(1)).join('')}`;
	if (!isIdentifierName(base) || ['decisions', 'assumptions', 'identities'].includes(base))
		base = `project${base.charAt(0).toUpperCase()}${base.slice(1)}`;
	let binding = base;
	for (let counter = 2; taken.has(binding); counter += 1) binding = `${base}${counter}`;
	taken.add(binding);
	return binding;
}

/** initiative.ts with every project the run leaves, in the plan's order. */
export function initiativeSource(input: InitiativeSourceInput): string {
	const taken = new Set<string>(['decisions', 'assumptions', 'identities']);
	const imports: string[] = [];
	const projects: string[] = [];
	for (const project of input.projects) {
		const binding = bindingFor(project.key, taken);
		imports.push(`import ${binding} from '${specifier(input.root, project.design)}';`);
		const key = isIdentifierName(project.key) ? project.key : `'${project.key}'`;
		projects.push(
			[
				`    ${key}: {`,
				`      design: ${binding},`,
				`      number: ${project.number},`,
				...(project.achieves ? [`      achieves: ${JSON.stringify(project.achieves)},`] : []),
				...(project.acceptance ? [`      acceptance: ${JSON.stringify(project.acceptance)},`] : []),
				'    },',
			].join('\n'),
		);
	}
	const shared: string[] = [];
	for (const [name, file] of [
		['decisions', input.decisions],
		['assumptions', input.assumptions],
		['identities', input.identities],
	] as const) {
		if (!file) continue;
		imports.push(`import ${name} from '${specifier(input.root, file)}';`);
		shared.push(`  ${name},`);
	}
	return `${imports.join('\n')}

export default {
  projects: {
${projects.join('\n')}
  },
${shared.join('\n')}${shared.length ? '\n' : ''}};
`;
}
