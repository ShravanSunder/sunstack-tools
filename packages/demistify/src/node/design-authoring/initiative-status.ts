import { basename } from 'node:path';

import { type InitiativeView, initiativeViewSchema } from '../../core/initiative-view.ts';
import { buildInitiative } from './check-initiative.ts';

interface InitiativeStatusOptions {
	readonly siteRoot?: string | undefined;
	readonly json?: boolean | undefined;
}

interface DeliverableStatus {
	readonly id: string;
	readonly title: string;
	readonly state: string;
	readonly issues: string;
	readonly checks: string;
	readonly blockedBy: readonly string[];
}

interface InitiativeStatus {
	readonly projects: readonly {
		readonly name: string;
		readonly number: number;
		readonly area?: string | undefined;
		readonly state: string;
		readonly reason: string;
		readonly achieves?: string | undefined;
		readonly acceptance?: string | undefined;
		readonly deliverables: readonly DeliverableStatus[];
	}[];
	readonly work: InitiativeView['work'];
}

function summarize(view: InitiativeView): InitiativeStatus {
	const deliverables = Object.values(view.deliverables);
	return {
		projects: view.order.map((name) => {
			const project = view.projects[name];
			if (!project) throw new Error(`${name}: project missing from the initiative view.`);
			return {
				name,
				number: project.number,
				area: project.area,
				state: project.state,
				reason: project.reason,
				achieves: project.achieves,
				acceptance: project.acceptance,
				deliverables: deliverables
					.filter(({ id }) => id.startsWith(`${name}/`))
					.map((deliverable): DeliverableStatus => {
						if (deliverable.kind === 'later')
							return {
								id: deliverable.id,
								title: deliverable.title,
								state: 'later',
								issues: '-',
								checks: '-',
								blockedBy: [],
							};
						const { issues } = deliverable.implementation;
						const { checks } = deliverable.verification;
						return {
							id: deliverable.id,
							title: deliverable.title,
							state: deliverable.state,
							issues: `${issues.filter(({ state }) => state === 'closed').length}/${issues.length}`,
							checks: `${checks.filter(({ state }) => state === 'passed').length}/${checks.length}`,
							blockedBy: deliverable.blockedBy,
						};
					}),
			};
		}),
		work: view.work,
	};
}

function formatStatus(status: InitiativeStatus): string {
	const lines: string[] = [];
	for (const project of status.projects) {
		lines.push(`${project.number} · ${project.name} · ${project.state} · ${project.reason}`);
		if (project.area) lines.push(`  Area: ${project.area}`);
		if (project.achieves) lines.push(`  Achieves: ${project.achieves}`);
		if (project.acceptance) lines.push(`  Owner checks: ${project.acceptance}`);
		for (const deliverable of project.deliverables)
			lines.push(
				`  ${deliverable.id} · ${deliverable.state} · issues ${deliverable.issues} · checks ${deliverable.checks} · ${deliverable.title}${
					deliverable.blockedBy.length ? ` · blocked by ${deliverable.blockedBy.join(', ')}` : ''
				}`,
			);
	}
	const list = (title: string, ids: readonly string[]): void => {
		lines.push(`${title}: ${ids.length ? ids.join(', ') : 'none'}`);
	};
	lines.push('');
	list('Ready now', status.work.readyNow);
	list('Needs a plan', status.work.needsPlan);
	list(
		'Blocks the most',
		status.work.blocksMost.map(({ id, waiting }) => `${id} (${waiting} waiting)`),
	);
	list('Built, not accepted', status.work.builtNotAccepted);
	return lines.join('\n');
}

/** Read an initiative's plan: projects by number, each deliverable's progress, and the work lists. */
export async function initiativeStatus(
	directory: string,
	options: InitiativeStatusOptions = {},
): Promise<string> {
	const build = await buildInitiative(directory, { siteRoot: options.siteRoot });
	const entry = [...build.generated].find(
		([file]) => basename(file) === 'initiative.generated.json',
	);
	if (!entry) throw new Error('The initiative check produced no initiative view.');
	const status = summarize(initiativeViewSchema.parse(entry[1]));
	return options.json ? JSON.stringify(status, null, 2) : formatStatus(status);
}
