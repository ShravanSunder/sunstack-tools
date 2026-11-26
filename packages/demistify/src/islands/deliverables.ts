import {
	type CoreDeliverableView,
	type DeliverableView,
	parseDeliverablesView,
} from '../core/deliverables-view.ts';
import {
	checkStateGlossary,
	deliverableStateGlossary,
	issueStateGlossary,
} from '../core/glossary.ts';
import { reportIssue } from '../core/issues.ts';
import { registerReviewSource, type ReviewSourceResult } from '../core/review-document.ts';
import { explainRecord, glossaryTerm } from './glossary.ts';

const deliveryStateLabels = {
	planned: 'Planned',
	later: 'Later',
	'implementation-in-progress': 'Implementation in progress',
	'design-blocked': 'Blocked by design',
	'awaiting-verification': 'Awaiting verification',
	'verification-blocked': 'Verification blocked',
	'verification-failed': 'Verification failed',
	'ready-for-acceptance': 'Ready for acceptance',
	accepted: 'Accepted',
	'dependency-blocked': 'Blocked on prerequisite',
} satisfies Record<DeliverableView['state'], string>;

export function deliveryState(state: DeliverableView['state']): HTMLButtonElement {
	return glossaryTerm(deliveryStateLabels[state], deliverableStateGlossary[state]);
}

const checkStateLabels = {
	missing: 'No result',
	stale: 'Older build',
	passed: 'Passed',
	failed: 'Failed',
	blocked: 'Blocked',
} satisfies Record<keyof typeof checkStateGlossary, string>;

function checkState(state: keyof typeof checkStateGlossary): HTMLButtonElement {
	return glossaryTerm(checkStateLabels[state], checkStateGlossary[state]);
}

export function linkTo(
	base: URL,
	reference: string,
	label: string,
	fragment?: string,
): HTMLAnchorElement {
	const link = document.createElement('a');
	const target = new URL(reference, base);
	if (fragment) target.hash = encodeURIComponent(fragment);
	link.href = target.href;
	link.textContent = label;
	return link;
}

export function deliveryDetails(
	item: CoreDeliverableView,
	source: URL,
	showAssignments = true,
): HTMLElement {
	const details = document.createElement('details');
	const summary = document.createElement('summary');
	summary.textContent = 'Checks and evidence';
	details.append(summary);
	const list = document.createElement('ul');
	const add = (label: string): HTMLLIElement => {
		const entry = document.createElement('li');
		entry.textContent = label;
		list.append(entry);
		return entry;
	};
	if (showAssignments) {
		if (item.implementer) add(`Implementer: ${item.implementer}`);
		if (item.verifier) add(`Verifier: ${item.verifier}`);
		if (item.reviewer) add(`Reviewer: ${item.reviewer}`);
		if (item.blockedBy.length) {
			const prerequisites = add('Blocked by ');
			for (const [index, id] of item.blockedBy.entries())
				prerequisites.append(index ? ', ' : '', id, explainRecord(id, 'deliverable'));
		}
	}
	if (item.implementation.candidate) add(`Candidate: ${item.implementation.candidate}`);
	if (item.verification.build) add(`Verification build: ${item.verification.build}`);
	for (const issue of item.implementation.issues)
		add(`${issue.title} · `).append(
			glossaryTerm(issue.state === 'closed' ? 'Closed' : 'Open', issueStateGlossary[issue.state]),
		);
	for (const check of item.verification.checks) {
		const reason = 'reason' in check ? check.reason : undefined;
		const entry = add(check.id);
		entry.append(explainRecord(check.id, 'planned-check'), ' · ', checkState(check.state));
		if (reason) entry.append(` · ${reason}`);
		if ('outcome' in check && check.outcome.kind !== 'blocked') {
			entry.append(` · ${check.outcome.observation}`);
			entry.append(' · ', linkTo(source, check.outcome.evidence, 'Evidence'));
		}
	}
	if (item.progress.state === 'accepted')
		add('').append(linkTo(source, item.progress.acceptance.evidence, 'Lead acceptance evidence'));
	for (const past of item.verification.history ?? []) {
		for (const [id, outcome] of Object.entries(past.outcomes)) {
			const entry = add(`Historical · ${past.build} · ${id}`);
			entry.append(explainRecord(id, 'planned-check'), ' · ', checkState('stale'));
			entry.append(` · Recorded result ${checkStateLabels[outcome.kind]}`);
			if (outcome.kind === 'blocked') entry.append(` · ${outcome.reason}`);
			else
				entry.append(` · ${outcome.observation} · `, linkTo(source, outcome.evidence, 'Evidence'));
		}
	}
	for (const [index, href] of item.implementation.evidence.entries())
		add('').append(linkTo(source, href, `Implementation evidence ${index + 1}`));
	for (const { id, label, destination } of [
		...item.links.rules,
		...item.links.designItems,
		...item.links.governingItems,
		...item.links.verificationChecks,
	]) {
		const title = `${id} · ${label}`;
		const value = destination
			? linkTo(source, destination.page, title, destination.fragment)
			: title;
		add('').append(value, explainRecord(id));
	}
	details.append(list);
	return details;
}

/** Read-only projection of the checked delivery graph for authored pages. */
export class DeliverablesIsland extends HTMLElement {
	private rendered: Promise<ReviewSourceResult> | undefined;

	connectedCallback(): void {
		if (this.closest('[data-inspector-copy], [data-inspector-content]')) return;
		this.rendered ??= this.renderDelivery();
		registerReviewSource(this, this.rendered);
	}

	private async renderDelivery(): Promise<ReviewSourceResult> {
		this.setAttribute('aria-busy', 'true');
		try {
			const reference = this.getAttribute('src')?.trim();
			if (!reference) throw new Error('Deliverables src must name generated delivery data.');
			const source = new URL(reference, this.baseURI);
			const response = await fetch(source);
			if (!response.ok) throw new Error(`HTTP ${response.status} loading ${reference}.`);
			const text = await response.text();
			const view = parseDeliverablesView(text);
			this.renderTable(Object.values(view.deliverables), source);
			this.dispatchEvent(new CustomEvent('island-ready', { bubbles: true }));
			return { kind: 'loaded', source: text };
		} catch (error) {
			this.textContent = 'Delivery progress unavailable.';
			this.setAttribute('role', 'status');
			reportIssue(this, error instanceof Error ? error.message : 'Invalid delivery data.');
			return { kind: 'failed' };
		} finally {
			this.removeAttribute('aria-busy');
		}
	}

	private renderTable(items: readonly DeliverableView[], source: URL): void {
		if (!items.length) {
			const summary = document.createElement('p');
			summary.textContent = 'No deliverables planned.';
			this.replaceChildren(summary);
			return;
		}
		const wrapper = document.createElement('div');
		wrapper.className = 'ex-table-scroll';
		const table = document.createElement('table');
		table.createCaption().textContent = this.getAttribute('label') ?? 'Deliverables';
		const header = table.createTHead().insertRow();
		for (const label of ['Deliverable', 'State', 'Work and evidence']) {
			const cell = document.createElement('th');
			cell.scope = 'col';
			cell.textContent = label;
			header.append(cell);
		}
		const body = table.createTBody();
		for (const item of items) {
			const row = body.insertRow();
			const title = document.createElement('th');
			title.scope = 'row';
			row.append(title);
			if (item.kind === 'later') {
				title.append(`${item.id} · ${item.title}`, explainRecord(item.id, 'deliverable'));
				row.insertCell().append(deliveryState('later'));
				row.insertCell();
				continue;
			}
			title.append(
				linkTo(
					source,
					item.links.plan.page,
					`${item.id} · ${item.title}`,
					item.links.plan.fragment,
				),
				explainRecord(item.id, 'deliverable'),
			);
			const state = row.insertCell();
			state.append(deliveryState(item.state));
			for (const reason of item.state === 'accepted' ? [] : item.reasons) {
				const paragraph = document.createElement('p');
				paragraph.textContent = reason;
				state.append(paragraph);
			}
			row.insertCell().append(deliveryDetails(item, source));
		}
		wrapper.append(table);
		this.replaceChildren(wrapper);
	}
}
