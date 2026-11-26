import { basename } from 'node:path';

import { isDecision } from '../../core/decision-record.ts';
import type { DeliverablesView } from '../../core/deliverables-view.ts';
import { decisionOutcome, projectDecisions } from '../../core/initiative-view.ts';
import {
	projectOverviewViewSchema,
	type ProjectOverviewView,
} from '../../core/project-overview-view.ts';
import { containsPath } from '../local-site.ts';
import { projectedDestinations, type RecordProjection } from './record-projection.ts';
import { recordLabel, recordRelationships } from './schema.ts';

const designDocumentFiles = [
	['requirements.html'],
	['specification.html'],
	['program-design.html'],
	['verification.html', 'verification-design.html'],
];

interface CheckedPage {
	readonly id: string;
	readonly title: string;
	readonly href: string;
	readonly file: string;
	readonly owned: boolean;
}

interface ProjectOverviewInput {
	readonly directory: string;
	readonly namespace: string | undefined;
	readonly pages: readonly CheckedPage[];
	readonly projection: RecordProjection;
	readonly delivery: DeliverablesView;
	readonly workLogs: Readonly<Record<string, string>> | undefined;
}

export function buildProjectOverview({
	directory,
	namespace,
	pages,
	projection,
	delivery,
	workLogs,
}: ProjectOverviewInput): ProjectOverviewView {
	const deliverables = Object.values(delivery.deliverables);
	const documents = designDocumentFiles.flatMap((filenames) => {
		const page = pages.find(({ file, owned }) => owned && filenames.includes(basename(file)));
		return page ? [{ title: page.title, href: page.href }] : [];
	});
	const pagesById = new Map(pages.map((page) => [page.id, page]));
	const datedLogs = Object.entries(workLogs ?? {})
		.map(([id, date]) => {
			const page = pagesById.get(id);
			if (!page) throw new Error(`Work log names an undeclared page: ${id}.`);
			return { title: page.title, href: page.href, date };
		})
		.toSorted((first, second) => second.date.localeCompare(first.date));

	const belongsToProject = (id: string): boolean => !namespace || id.startsWith(`${namespace}/`);
	const destinations = projectedDestinations(projection, directory);
	const governing = new Set(
		deliverables.flatMap((item) =>
			item.kind === 'core' ? item.links.governingItems.map(({ id }) => id) : [],
		),
	);
	const records = [...projection.records.values()].map((record) => ({
		id: record.id,
		kind: record.kind,
		authority: isDecision(record) ? record.authority : undefined,
		supersedes: isDecision(record) ? record.supersedes : undefined,
		title: recordLabel(record),
		statement: record.kind === 'ruling' ? record.statement : undefined,
		relationships: recordRelationships(record),
	}));
	const decisions = projectDecisions(records).flatMap(({ record, status, governedRecords }) => {
		if (status === 'Superseded') return [];
		const authored = projection.records.get(record.id);
		const needsReview =
			isDecision(authored) &&
			authored.authority.state === 'agent-decided' &&
			authored.authority.ownerReview === undefined;
		if (status !== 'Open' && !needsReview) return [];
		const placement = projection.placements.get(record.id);
		const destination = destinations.get(record.id);
		if (!placement || !destination) return [];
		if (
			!belongsToProject(record.id) &&
			!containsPath(directory, placement.file) &&
			!governing.has(record.id) &&
			!governedRecords.some(({ id }) => belongsToProject(id))
		)
			return [];
		return [
			{
				title: `${record.id} · ${record.title}`,
				href: `${destination.page}#${encodeURIComponent(destination.fragment)}`,
				status: status === 'Open' ? 'Open' : 'Needs review',
				outcome: decisionOutcome(record),
			},
		];
	});

	return projectOverviewViewSchema.parse({
		version: 1,
		deliverables: deliverables.map((item) =>
			item.kind === 'core'
				? {
						kind: item.kind,
						id: item.id,
						title: item.title,
						state: item.state,
						destination: item.links.plan,
					}
				: item,
		),
		documents,
		decisions,
		workLogs: datedLogs,
	});
}
