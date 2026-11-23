import type { DeliverableView } from '../core/deliverables-view.ts';
import { reportIssue } from '../core/issues.ts';
import {
	parseProjectView,
	projectDecisions,
	type ProjectDecision,
	type ProjectRecord,
	type ProjectView,
} from '../core/project-view.ts';
import { registerReviewSource, type ReviewSourceResult } from '../core/review-document.ts';
import { deliveryDetails, deliveryStateLabels, linkTo } from './deliverables.ts';

function appendHeading(parent: HTMLElement, level: 'h2' | 'h3', title: string): HTMLElement {
	const heading = document.createElement(level);
	heading.textContent = title;
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

function recordLink(
	source: URL,
	record: ProjectRecord,
	title = record.title,
): HTMLAnchorElement | string {
	const label = `${record.id} · ${title}`;
	const destination = record.destination;
	return destination ? linkTo(source, destination.page, label, destination.fragment) : label;
}

function renderDecision(list: HTMLUListElement, source: URL, decision: ProjectDecision): void {
	const { record, status, authority, replacement, governedRecords } = decision;
	const item = appendItem(list);
	item.append(
		recordLink(source, record, status === 'Open' ? record.authority?.question : undefined),
		` · Authority: ${authority} · Status: ${status}`,
	);
	if (record.authority?.decidedOn) item.append(` · ${record.authority.decidedOn}`);
	if (record.authority?.decision) {
		const outcome = document.createElement('p');
		outcome.textContent = record.authority.decision;
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

const assumptionStatuses: ReadonlyMap<string, string> = new Map([
	['open', 'Unverified'],
	['confirmed', 'Confirmed'],
	['refuted', 'Refuted'],
]);

function renderDelivery(
	parent: HTMLLIElement,
	delivery: DeliverableView,
	view: ProjectView,
	source: URL,
): void {
	const state = document.createElement('strong');
	state.textContent = deliveryStateLabels[delivery.state];
	if (delivery.kind === 'later') {
		parent.append(delivery.title, ' · ', state);
		return;
	}
	parent.append(
		linkTo(source, delivery.links.plan.page, delivery.title, delivery.links.plan.fragment),
		' · ',
		state,
	);
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
			item.append('Prerequisite ');
			item.append(
				required?.kind === 'core'
					? linkTo(source, required.links.plan.page, id, required.links.plan.fragment)
					: id,
			);
		}
	}
	parent.append(deliveryDetails(delivery, source, false));
}

/** Project data links to authored pages and shares the delivery evidence view. */
export class ProjectOverviewIsland extends HTMLElement {
	private rendered: Promise<ReviewSourceResult> | undefined;

	connectedCallback(): void {
		if (this.closest('[data-inspector-copy], [data-inspector-content]')) return;
		this.rendered ??= this.renderProject();
		registerReviewSource(this, this.rendered);
	}

	private async renderProject(): Promise<ReviewSourceResult> {
		this.setAttribute('aria-busy', 'true');
		try {
			const reference = this.getAttribute('src')?.trim();
			if (!reference) throw new Error('Project overview src must name generated project data.');
			const source = new URL(reference, this.baseURI);
			const response = await fetch(source);
			if (!response.ok) throw new Error(`HTTP ${response.status} loading ${reference}.`);
			const text = await response.text();
			this.renderOverview(parseProjectView(text), source);
			this.dispatchEvent(new CustomEvent('island-ready', { bubbles: true }));
			return { kind: 'loaded', source: text };
		} catch (error) {
			this.textContent = 'Project progress unavailable.';
			this.setAttribute('role', 'status');
			reportIssue(this, error instanceof Error ? error.message : 'Invalid project data.');
			return { kind: 'failed' };
		} finally {
			this.removeAttribute('aria-busy');
		}
	}

	private renderOverview(view: ProjectView, source: URL): void {
		const content = document.createElement('div');
		const decisions = projectDecisions(view.records);
		const questions = decisions.filter(({ status }) => status === 'Open');
		if (questions.length) {
			appendHeading(content, 'h2', 'Waiting on owner');
			const list = appendList(content);
			for (const decision of questions) renderDecision(list, source, decision);
		}

		appendHeading(content, 'h2', 'Milestones');
		const deliveries = Object.values(view.deliverables);
		const designs = Object.entries(view.designs);
		if (!designs.length) content.append('No milestones registered.');
		for (const [name, milestone] of designs) {
			const heading = appendHeading(content, 'h3', '');
			const title = milestone.title ?? name;
			heading.append(milestone.firstPage ? linkTo(source, milestone.firstPage, title) : title);
			if (milestone.lead) heading.append(` · Lead ${milestone.lead}`);
			const planned = deliveries.filter(({ id }): boolean => id.startsWith(`${name}/`));
			if (!planned.length) continue;
			const list = appendList(content);
			for (const delivery of planned) renderDelivery(appendItem(list), delivery, view, source);
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
				if (status) item.append(` · ${status}`);
			}
		}

		const otherRecords = view.records.filter(
			({ kind }) => kind !== 'choice' && kind !== 'assumption',
		);
		if (otherRecords.length) {
			appendHeading(content, 'h2', 'Design records');
			const list = appendList(content);
			for (const record of otherRecords) appendItem(list).append(recordLink(source, record));
		}
		this.replaceChildren(content);
	}
}
