import { realpath } from 'node:fs/promises';
import { resolve } from 'node:path';

import { containsPath } from '../local-site.ts';
import { loadDesignSet } from './check-design.ts';
import { loadProject } from './check-project.ts';
import { governingRecords, projectDelivery, realizationNotes } from './delivery-model.ts';
import { findDesignInput } from './design-location.ts';
import { recordStatement, type DesignRecord } from './schema.ts';

function briefRecord(record: DesignRecord): string[] {
	const statement = recordStatement(record);
	return [
		`- **${record.id}** ${statement}`,
		...(record.kind === 'planned-check'
			? [`  - Observation: ${record.journey}`, `  - Unverified: ${record.unverified}`]
			: []),
		...('cases' in record ? (record.cases ?? []) : []).map(
			({ given, expect }): string => `  - Given ${given}\n    Expect ${expect}`,
		),
	];
}

function section(heading: string, items: readonly string[]): string[] {
	return items.length ? ['', `## ${heading}`, '', ...items] : [];
}

export async function deliveryBrief(
	reference: string,
	directory: string,
	siteRoot?: string,
): Promise<string> {
	const start = await realpath(resolve(directory));
	const limit = siteRoot === undefined ? undefined : await realpath(resolve(siteRoot));
	if (limit && !containsPath(limit, start))
		throw new Error('The deliverable directory is outside the site root.');
	const [project, milestone] = await Promise.all([
		findDesignInput(start, 'project.ts', limit),
		findDesignInput(start, 'design.ts', limit),
	]);
	const projectModel = project ? await loadProject(project, siteRoot) : undefined;
	const namespace = milestone && projectModel?.milestones.get(milestone);
	const model = projectModel ?? (await loadDesignSet(milestone ?? start, siteRoot));
	const delivery = projectDelivery(model.design, model.entries);
	await model.check.verify();
	const scopedId = namespace && !reference.includes('/') ? `${namespace}/${reference}` : reference;
	const matches = model.entries.filter(
		({ deliverable: { id } }): boolean =>
			id === scopedId || (!namespace && !reference.includes('/') && id.endsWith(`/${reference}`)),
	);
	if (matches.length > 1)
		throw new Error(
			`Deliverable ${reference} is ambiguous. Use ${matches.map(({ deliverable }) => deliverable.id).join(', ')}.`,
		);
	const [entry] = matches;
	if (!entry) throw new Error(`Unknown deliverable: ${scopedId}.`);
	const { deliverable, dependsOn } = entry;
	if (deliverable.kind !== 'core')
		throw new Error(`${deliverable.id} is a later deliverable with no brief.`);
	const { scope } = deliverable;
	const records = new Map(model.design.records.map((record) => [record.id, record]));
	const selected = (ids: readonly string[]): DesignRecord[] =>
		ids.flatMap((id) => {
			const record = records.get(id);
			return record ? [record] : [];
		});
	const governance = governingRecords(deliverable, records);
	const coverage = delivery.coverage.get(deliverable.id);

	return [
		`# ${deliverable.id} · ${deliverable.title}`,
		...section(
			'Prerequisites',
			dependsOn.map((id): string => `- ${id}`),
		),
		...section('Needs', selected(scope.needs).flatMap(briefRecord)),
		...section('Rules', selected(scope.rules).flatMap(briefRecord)),
		...section('Realizations', selected(scope.designItems).flatMap(briefRecord)),
		...section('Checks', selected(scope.verificationChecks).flatMap(briefRecord)),
		...section(
			'Tasks',
			scope.implementationTasks.map((id): string => `- ${id}`),
		),
		...section(
			'Open owner decisions',
			governance
				.filter((record) => record.kind === 'choice' && record.authority.state === 'owner-open')
				.flatMap(briefRecord),
		),
		...section(
			'Open assumptions',
			governance
				.filter((record) => record.kind === 'assumption' && record.status === 'open')
				.flatMap(briefRecord),
		),
		...section(
			'Coverage gaps',
			(coverage?.gaps ?? []).map((gap): string => `- ${gap}`),
		),
		...section(
			'Design notes',
			realizationNotes(coverage ? [coverage] : []).map((note): string => `- ${note}`),
		),
	].join('\n');
}
