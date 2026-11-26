import type { DeliverableView } from '../core/deliverables-view.ts';
import {
	assumptionStateGlossary,
	decisionStatusGlossary,
	levelGlossary,
	projectStateGlossary,
} from '../core/glossary.ts';
import { createIconButton } from '../core/icons.ts';
import {
	decisionOutcome,
	parseInitiativeView,
	projectDecisions,
	type ProjectDecision,
	type ProjectRecord,
	type InitiativeView,
} from '../core/initiative-view.ts';
import { reportIssue } from '../core/issues.ts';
import { registerReviewSource, type ReviewSourceResult } from '../core/review-document.ts';
import { deliveryDetails, deliveryState, linkTo } from './deliverables.ts';
import { openDrawer } from './drawer.ts';
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
	const answer = decisionOutcome(record);
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
	const details = deliveryDetails(delivery, source, false);
	const context = document.createElement('div');
	if (delivery.builder) {
		const builder = document.createElement('p');
		builder.textContent = `Builder ${delivery.builder}`;
		context.append(builder);
	}
	for (const reason of delivery.state === 'accepted' ? [] : delivery.reasons) {
		const paragraph = document.createElement('p');
		paragraph.textContent = reason;
		context.append(paragraph);
	}
	if (delivery.dependsOn.length) {
		const list = appendList(context);
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
	if (context.hasChildNodes()) details.firstElementChild?.after(context);
	parent.append(details);
}

const projectStateLabels = {
	accepted: 'Accepted',
	'dependency-blocked': 'Blocked on prerequisite',
	'design-blocked': 'Blocked by design',
	'in-progress': 'In progress',
	planned: 'Planned',
	later: 'Later',
} satisfies Record<InitiativeView['projects'][string]['state'], string>;

type InitiativeProject = InitiativeView['projects'][string];

function projectStatus(project: InitiativeProject): HTMLParagraphElement {
	const status = document.createElement('p');
	status.className = 'ex-initiative-project-state';
	if (project.lead) status.append(`Lead ${project.lead} · `);
	status.append(
		glossaryTerm(projectStateLabels[project.state], projectStateGlossary[project.state]),
	);
	return status;
}

function projectOwnerQuestions(
	namespace: string,
	questions: readonly ProjectDecision[],
	deliverables: readonly DeliverableView[],
): readonly ProjectDecision[] {
	const governingIds = new Set(
		deliverables.flatMap((delivery) =>
			delivery.kind === 'core' ? delivery.links.governingItems.map(({ id }) => id) : [],
		),
	);
	return questions.filter(
		({ record, governedRecords }) =>
			record.id.startsWith(namespace) ||
			governingIds.has(record.id) ||
			governedRecords.some(({ id }) => id.startsWith(namespace)),
	);
}

function renderProjectSummary(
	body: HTMLElement,
	project: InitiativeProject,
	deliverables: readonly DeliverableView[],
	questions: readonly ProjectDecision[],
	view: InitiativeView,
	source: URL,
): void {
	body.append(projectStatus(project));
	if (project.achieves) {
		const achieves = document.createElement('p');
		achieves.textContent = project.achieves;
		body.append(achieves);
	}
	if (project.acceptance) {
		const acceptance = document.createElement('p');
		const label = document.createElement('strong');
		label.textContent = 'What the owner checks';
		acceptance.append(label, ' ', project.acceptance);
		body.append(acceptance);
	}
	const reason = document.createElement('p');
	reason.textContent = project.reason;
	body.append(reason);
	if (project.firstPage) {
		const designPage = linkTo(source, project.firstPage, 'Open design page');
		designPage.className = 'ex-design-navigation-action';
		body.append(designPage);
	}

	appendHeading(body, 'h3', 'Open owner decisions');
	if (questions.length) {
		const list = appendList(body);
		for (const { record } of questions) {
			const question = record.question ?? record.authority?.question;
			appendItem(list).append(recordLink(source, record, question));
		}
	} else {
		const empty = document.createElement('p');
		empty.textContent = 'No open owner decisions.';
		body.append(empty);
	}

	appendHeading(body, 'h3', 'Deliverables');
	if (deliverables.length) {
		const list = appendList(body);
		for (const delivery of deliverables) renderDelivery(appendItem(list), delivery, view, source);
	} else {
		const empty = document.createElement('p');
		empty.textContent = 'No deliverables planned.';
		body.append(empty);
	}
}

function openProjectSummary(
	view: InitiativeView,
	source: URL,
	projectName: string,
	questions: readonly ProjectDecision[],
	trigger: HTMLButtonElement,
): () => void {
	const project = view.projects[projectName];
	if (!project) throw new Error(`Generated initiative names unknown project ${projectName}.`);
	const title = project.title ?? projectName;
	const namespace = `${projectName}/`;
	const deliverables = Object.values(view.deliverables).filter(({ id }) =>
		id.startsWith(namespace),
	);
	const ownerQuestions = projectOwnerQuestions(namespace, questions, deliverables);
	trigger.focus({ preventScroll: true });
	return openDrawer(title, (close) => {
		const header = document.createElement('header');
		header.className = 'ex-inspect-header ex-initiative-project-header';
		const heading = document.createElement('strong');
		heading.textContent = title;
		header.append(heading, createIconButton('x', 'Close project summary', close));
		const body = document.createElement('div');
		body.className = 'ex-inspect-body ex-initiative-project-summary';
		renderProjectSummary(body, project, deliverables, ownerQuestions, view, source);
		body.addEventListener('click', (event): void => {
			if (
				!event.defaultPrevented &&
				event.button === 0 &&
				!event.metaKey &&
				!event.ctrlKey &&
				!event.altKey &&
				!event.shiftKey &&
				event.target instanceof Element &&
				event.target.closest('a[href]')
			)
				close();
		});
		trigger.setAttribute('aria-expanded', 'true');
		return {
			header,
			body,
			presentation: { kind: 'standard' },
			restore: (): void => trigger.setAttribute('aria-expanded', 'false'),
		};
	});
}

function renderProjects(
	content: HTMLElement,
	view: InitiativeView,
	toggleProjectSummary: (name: string, trigger: HTMLButtonElement) => void,
): void {
	appendHeading(content, 'h2', 'Projects', levelGlossary.project);
	const introduction = document.createElement('p');
	introduction.textContent = levelGlossary.initiative;
	content.append(introduction);
	if (!view.order.length) {
		content.append('No projects registered.');
		return;
	}
	const projectGroups = new Map<number, HTMLElement[]>();
	for (const name of view.order) {
		const project = view.projects[name];
		if (!project) throw new Error(`Generated initiative order names unknown project ${name}.`);
		const section = document.createElement('section');
		section.className = 'ex-initiative-project';
		const trigger = document.createElement('button');
		trigger.type = 'button';
		trigger.className = 'ex-initiative-project-trigger';
		trigger.textContent = project.title ?? name;
		trigger.setAttribute('aria-expanded', 'false');
		trigger.addEventListener('click', (): void => toggleProjectSummary(name, trigger));
		const heading = appendHeading(section, 'h4', '');
		heading.setAttribute('aria-label', trigger.textContent);
		heading.append(trigger);
		section.append(projectStatus(project));
		const projects = projectGroups.get(project.number) ?? [];
		projects.push(section);
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
	const { readyNow, needsPlan, blocksMost, builtNotAccepted } = view.work;
	const groups = [
		{ title: 'Ready now', entries: readyNow.map((id) => ({ id, detail: '' })) },
		{
			title: 'Needs a plan',
			explanation: 'No implementation steps yet: write its plan before it can start.',
			entries: needsPlan.map((id) => ({ id, detail: '' })),
		},
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
	for (const { title, explanation, entries } of groups) {
		const group = document.createElement('section');
		appendHeading(group, 'h3', title, explanation);
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
	private closeProjectSummary: (() => void) | undefined;

	connectedCallback(): void {
		if (this.closest('[data-inspector-copy], [data-inspector-content]')) return;
		this.rendered ??= this.renderInitiative();
		registerReviewSource(this, this.rendered);
	}

	disconnectedCallback(): void {
		this.closeProjectSummary?.();
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
		const decisions = projectDecisions(view.records);
		const questions = decisions.filter(({ status }) => status === 'Open');
		renderProjects(content, view, (name, trigger): void => {
			if (trigger.getAttribute('aria-expanded') === 'true') {
				this.closeProjectSummary?.();
				return;
			}
			this.closeProjectSummary = openProjectSummary(view, source, name, questions, trigger);
		});
		renderWorkNow(content, view, source);
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
