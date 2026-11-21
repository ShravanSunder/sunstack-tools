import type { DeliverableView } from '../core/deliverables-view.ts';
import { reportIssue } from '../core/issues.ts';
import { ownerQuestions, parseProjectView, type ProjectView } from '../core/project-view.ts';
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

function renderDelivery(
	parent: HTMLLIElement,
	delivery: DeliverableView,
	view: ProjectView,
	source: URL,
): void {
	const state = document.createElement('strong');
	state.textContent = deliveryStateLabels[delivery.state];
	parent.append(
		linkTo(source, delivery.links.plan.page, delivery.title, delivery.links.plan.fragment),
		' · ',
		state,
	);
	if (delivery.builder) parent.append(` · Builder ${delivery.builder}`);
	for (const reason of delivery.reasons) {
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
				required ? linkTo(source, required.links.plan.page, id, required.links.plan.fragment) : id,
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
		const questions = ownerQuestions(view.records);
		if (questions.length) {
			appendHeading(content, 'h2', 'Waiting on owner');
			const list = appendList(content);
			for (const { id, title, destination, authority } of questions)
				appendItem(list).append(
					linkTo(
						source,
						destination.page,
						`${id} · ${authority?.question ?? title}`,
						destination.fragment,
					),
				);
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

		if (view.records.length) {
			appendHeading(content, 'h2', 'Design records');
			const list = appendList(content);
			for (const { id, title, destination } of view.records)
				appendItem(list).append(
					linkTo(source, destination.page, `${id} · ${title}`, destination.fragment),
				);
		}
		this.replaceChildren(content);
	}
}
