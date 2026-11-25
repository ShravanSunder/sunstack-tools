import type { DeliverableView } from '../core/deliverables-view.ts';
import {
	assumptionStateGlossary,
	decisionStatusGlossary,
	levelGlossary,
	projectStateGlossary,
} from '../core/glossary.ts';
import {
	parseInitiativeView,
	projectDecisions,
	type ProjectDecision,
	type ProjectRecord,
	type InitiativeView,
} from '../core/initiative-view.ts';
import { reportIssue } from '../core/issues.ts';
import { registerReviewSource, type ReviewSourceResult } from '../core/review-document.ts';
import { deliveryDetails, deliveryState, linkTo } from './deliverables.ts';
import { explainRecord, glossaryTerm } from './glossary.ts';

function appendHeading(
	parent: HTMLElement,
	level: 'h2' | 'h3' | 'h4',
	title: string,
	explanation?: string,
): HTMLElement {
	const heading = document.createElement(level);
	heading.textContent = title;
	if (explanation) {
		heading.setAttribute('aria-label', title);
		heading.append(glossaryTerm(title, explanation, true));
	}
	parent.append(heading);
	return heading;
}

function appendList(parent: HTMLElement): HTMLUListElement {
	const list = document.createElement('ul');
	parent.append(list);
	return list;
}

function appendItem(list: HTMLUListElement): HTMLLIElement {
	const item = document.createElement('li');
	list.append(item);
	return item;
}

function recordLink(source: URL, record: ProjectRecord, title = record.title): HTMLElement {
	const label = `${record.id} · ${title}`;
	const destination = record.destination;
	const value = document.createElement('span');
	value.append(
		destination ? linkTo(source, destination.page, label, destination.fragment) : label,
		explainRecord(record.id, record.kind),
	);
	return value;
}

function renderDecision(list: HTMLUListElement, source: URL, decision: ProjectDecision): void {
	const { record, status, authority, replacement, governedRecords } = decision;
	const item = appendItem(list);
	item.append(
		recordLink(
			source,
			record,
			status === 'Open' ? (record.question ?? record.authority?.question) : undefined,
		),
		` · Authority: ${authority} · Status: `,
		glossaryTerm(status, decisionStatusGlossary[status]),
	);
	if (record.authority?.decidedOn) item.append(` · ${record.authority.decidedOn}`);
	const answer =
		record.authority?.decision ??
		(record.kind === 'ruling' && record.title !== record.statement ? record.statement : undefined);
	if (answer) {
		const outcome = document.createElement('p');
		outcome.textContent = answer;
		item.append(outcome);
	}
	if (replacement) {
		const link = document.createElement('p');
		link.append('Replaced by ', recordLink(source, replacement));
		item.append(link);
	}
	if (governedRecords.length) {
		const details = document.createElement('details');
		const summary = document.createElement('summary');
		summary.textContent = 'Affected design records';
		details.append(summary);
		const links = appendList(details);
		for (const governed of governedRecords) appendItem(links).append(recordLink(source, governed));
		item.append(details);
	}
}

const assumptionStatuses = new Map<string, readonly [string, string]>([
	['open', ['Unverified', assumptionStateGlossary.open]],
	['confirmed', ['Confirmed', assumptionStateGlossary.confirmed]],
	['refuted', ['Refuted', assumptionStateGlossary.refuted]],
]);

function appendDeliverableLink(
	parent: HTMLElement,
	source: URL,
	id: string,
	delivery: DeliverableView | undefined,
): void {
	const label = delivery ? `${id} · ${delivery.title}` : id;
	parent.append(
		delivery?.kind === 'core'
			? linkTo(source, delivery.links.plan.page, label, delivery.links.plan.fragment)
			: label,
		explainRecord(id, 'deliverable'),
	);
}

function renderDelivery(
	parent: HTMLLIElement,
	delivery: DeliverableView,
	view: InitiativeView,
	source: URL,
): void {
	appendDeliverableLink(parent, source, delivery.id, delivery);
	parent.append(' · ', deliveryState(delivery.state));
	if (delivery.kind === 'later') return;
	if (delivery.builder) parent.append(` · Builder ${delivery.builder}`);
	for (const reason of delivery.state === 'accepted' ? [] : delivery.reasons) {
		const paragraph = document.createElement('p');
		paragraph.textContent = reason;
		parent.append(paragraph);
	}
	if (delivery.dependsOn.length) {
		const list = appendList(parent);
		for (const id of delivery.dependsOn) {
			const required = view.deliverables[id];
			const item = appendItem(list);
			item.append(glossaryTerm('Prerequisite', levelGlossary.dependsOn), ' ');
			item.append(
				required?.kind === 'core'
					? linkTo(source, required.links.plan.page, id, required.links.plan.fragment)
					: id,
				explainRecord(id, 'deliverable'),
			);
		}
	}
	parent.append(deliveryDetails(delivery, source, false));
}

const projectStateLabels = {
	accepted: 'Accepted',
	'dependency-blocked': 'Blocked on prerequisite',
	'design-blocked': 'Blocked by design',
	'in-progress': 'In progress',
	planned: 'Planned',
	later: 'Later',
} satisfies Record<InitiativeView['projects'][string]['state'], string>;

function projectLink(
	source: URL,
	name: string,
	project: InitiativeView['projects'][string],
): HTMLAnchorElement | string {
	const title = project.title ?? name;
	return project.firstPage ? linkTo(source, project.firstPage, title) : title;
}

function renderProject(
	name: string,
	project: InitiativeView['projects'][string],
	deliverables: readonly DeliverableView[],
	view: InitiativeView,
	source: URL,
): HTMLElement {
	const section = document.createElement('section');
	section.className = 'ex-initiative-project';
	appendHeading(section, 'h4', '').append(projectLink(source, name, project));
	const status = document.createElement('p');
	status.className = 'ex-initiative-project-state';
	if (project.lead) status.append(`Lead ${project.lead} · `);
	status.append(
		glossaryTerm(projectStateLabels[project.state], projectStateGlossary[project.state]),
	);
	section.append(status);
	if (project.achieves) {
		const achieves = document.createElement('p');
		achieves.textContent = project.achieves;
		section.append(achieves);
	}
	if (project.acceptance) {
		const acceptance = document.createElement('p');
		const label = document.createElement('strong');
		label.textContent = 'What the owner checks';
		acceptance.append(label, ' ', project.acceptance);
		section.append(acceptance);
	}
	const reason = document.createElement('p');
	reason.textContent = project.reason;
	section.append(reason);
	if (deliverables.length) {
		const list = appendList(section);
		for (const delivery of deliverables) renderDelivery(appendItem(list), delivery, view, source);
	}
	return section;
}

function renderProjects(content: HTMLElement, view: InitiativeView, source: URL): void {
	appendHeading(content, 'h2', 'Projects', levelGlossary.project);
	const introduction = document.createElement('p');
	introduction.textContent = levelGlossary.initiative;
	content.append(introduction);
	if (!view.order.length) {
		content.append('No projects registered.');
		return;
	}
	const deliveries = new Map<string, DeliverableView[]>();
	for (const item of Object.values(view.deliverables)) {
		const name = item.id.split('/', 1)[0] ?? '';
		const projectDeliveries = deliveries.get(name) ?? [];
		projectDeliveries.push(item);
		deliveries.set(name, projectDeliveries);
	}
	const projectGroups = new Map<number, HTMLElement[]>();
	for (const name of view.order) {
		const project = view.projects[name];
		if (!project) throw new Error(`Generated initiative order names unknown project ${name}.`);
		const projects = projectGroups.get(project.number) ?? [];
		projects.push(renderProject(name, project, deliveries.get(name) ?? [], view, source));
		projectGroups.set(project.number, projects);
	}
	for (const [number, projects] of projectGroups) {
		const group = document.createElement('section');
		const heading = projects.length > 1 ? `${number} · side by side` : String(number);
		appendHeading(group, 'h3', heading, levelGlossary.number);
		const layout = document.createElement('div');
		layout.className = 'ex-initiative-project-group';
		layout.append(...projects);
		group.append(layout);
		content.append(group);
	}
}

function renderWorkNow(content: HTMLElement, view: InitiativeView, source: URL): void {
	const { readyNow, blocksMost, builtNotAccepted } = view.work;
	const groups = [
		{ title: 'Ready now', entries: readyNow.map((id) => ({ id, detail: '' })) },
		{
			title: 'Blocks the most',
			entries: blocksMost.map(({ id, waiting }) => ({
				id,
				detail: ` · ${waiting} ${waiting === 1 ? 'deliverable' : 'deliverables'} waiting`,
			})),
		},
		{ title: 'Built, not accepted', entries: builtNotAccepted.map((id) => ({ id, detail: '' })) },
	].filter(({ entries }) => entries.length > 0);
	if (!groups.length) return;
	const section = document.createElement('section');
	appendHeading(section, 'h2', 'Work now');
	for (const { title, entries } of groups) {
		const group = document.createElement('section');
		appendHeading(group, 'h3', title);
		const list = appendList(group);
		for (const { id, detail } of entries) {
			const item = appendItem(list);
			appendDeliverableLink(item, source, id, view.deliverables[id]);
			if (detail) item.append(detail);
		}
		section.append(group);
	}
	content.append(section);
}

/** Initiative data links to authored projects and shares their delivery evidence. */
export class InitiativeOverviewIsland extends HTMLElement {
	private rendered: Promise<ReviewSourceResult> | undefined;

	connectedCallback(): void {
		if (this.closest('[data-inspector-copy], [data-inspector-content]')) return;
		this.rendered ??= this.renderInitiative();
		registerReviewSource(this, this.rendered);
	}

	private async renderInitiative(): Promise<ReviewSourceResult> {
		this.setAttribute('aria-busy', 'true');
		try {
			const reference = this.getAttribute('src')?.trim();
			if (!reference)
				throw new Error('Initiative overview src must name generated initiative data.');
			const source = new URL(reference, this.baseURI);
			const response = await fetch(source);
			if (!response.ok) throw new Error(`HTTP ${response.status} loading ${reference}.`);
			const text = await response.text();
			this.renderOverview(parseInitiativeView(text), source);
			this.dispatchEvent(new CustomEvent('island-ready', { bubbles: true }));
			return { kind: 'loaded', source: text };
		} catch (error) {
			this.textContent = 'Initiative progress unavailable.';
			this.setAttribute('role', 'status');
			reportIssue(this, error instanceof Error ? error.message : 'Invalid initiative data.');
			return { kind: 'failed' };
		} finally {
			this.removeAttribute('aria-busy');
		}
	}

	private renderOverview(view: InitiativeView, source: URL): void {
		const content = document.createElement('div');
		renderProjects(content, view, source);
		renderWorkNow(content, view, source);
		const decisions = projectDecisions(view.records);
		const questions = decisions.filter(({ status }) => status === 'Open');
		if (questions.length) {
			appendHeading(content, 'h2', 'Waiting on owner');
			const list = appendList(content);
			for (const decision of questions) renderDecision(list, source, decision);
		}

		const decided = decisions.filter(({ status }) => status !== 'Open');
		if (decided.length) {
			appendHeading(content, 'h2', 'Decisions');
			const list = appendList(content);
			for (const decision of decided) renderDecision(list, source, decision);
		}

		const assumptions = view.records.filter(({ kind }) => kind === 'assumption');
		if (assumptions.length) {
			appendHeading(content, 'h2', 'Assumptions');
			const list = appendList(content);
			for (const assumption of assumptions) {
				const item = appendItem(list);
				item.append(recordLink(source, assumption));
				const status = assumptionStatuses.get(assumption.status ?? '');
				if (status) item.append(' · ', glossaryTerm(...status));
			}
		}

		const otherRecords = view.records.filter(
			({ kind }) => kind !== 'choice' && kind !== 'ruling' && kind !== 'assumption',
		);
		if (otherRecords.length) {
			appendHeading(content, 'h2', 'Design records');
			const list = appendList(content);
			for (const record of otherRecords) appendItem(list).append(recordLink(source, record));
		}
		this.replaceChildren(content);
	}
}
