import { type DeliverableView, parseDeliverablesView } from '../core/deliverables-view.ts';
import { reportIssue } from '../core/issues.ts';
import { registerReviewSource, type ReviewSourceResult } from '../core/review-document.ts';

export const deliveryStateLabels = {
	planned: 'Planned',
	'implementation-in-progress': 'Implementation in progress',
	'design-blocked': 'Blocked by design',
	'awaiting-verification': 'Awaiting verification',
	'verification-blocked': 'Verification blocked',
	'verification-failed': 'Verification failed',
	'ready-for-acceptance': 'Ready for acceptance',
	accepted: 'Accepted',
	'dependency-blocked': 'Blocked on prerequisite',
} satisfies Record<DeliverableView['state'], string>;

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
	item: DeliverableView,
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
		if (item.builder) add(`Builder: ${item.builder}`);
		if (item.dependsOn.length) add(`Prerequisites: ${item.dependsOn.join(', ')}`);
	}
	if (item.implementation.candidate) add(`Candidate: ${item.implementation.candidate}`);
	if (item.verification.build) add(`Verification build: ${item.verification.build}`);
	for (const task of item.implementation.tasks) add(`${task.id} · ${task.state}`);
	for (const check of item.verification.checks) {
		const reason = 'reason' in check ? check.reason : undefined;
		const entry = add(`${check.id} · ${check.state}${reason ? ` · ${reason}` : ''}`);
		if ('outcome' in check && check.outcome.kind !== 'blocked') {
			entry.append(` · ${check.outcome.observation}`);
			entry.append(' · ', linkTo(source, check.outcome.evidence, 'Evidence'));
		}
	}
	if (item.progress.state === 'accepted')
		add('').append(linkTo(source, item.progress.acceptance.evidence, 'Lead acceptance evidence'));
	for (const past of item.verification.history ?? []) {
		for (const [id, outcome] of Object.entries(past.outcomes)) {
			const entry = add(`Historical · ${past.build} · ${id} · ${outcome.kind}`);
			if (outcome.kind === 'blocked') entry.append(` · ${outcome.reason}`);
			else
				entry.append(` · ${outcome.observation} · `, linkTo(source, outcome.evidence, 'Evidence'));
		}
	}
	for (const [index, href] of item.implementation.evidence.entries())
		add('').append(linkTo(source, href, `Implementation evidence ${index + 1}`));
	for (const link of [
		...item.links.rules,
		...item.links.designItems,
		...item.links.governingItems,
		...item.links.verificationChecks,
	])
		add('').append(linkTo(source, link.page, link.id, link.fragment));
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
			this.renderTable(Object.values(view.deliverables), source, view.snapshotRevision);
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

	private renderTable(
		items: readonly DeliverableView[],
		source: URL,
		snapshotRevision: string,
	): void {
		const snapshot = document.createElement('small');
		snapshot.textContent = `Last checked snapshot ${snapshotRevision.slice(0, 12)}`;
		snapshot.title = snapshotRevision;
		if (!items.length) {
			const summary = document.createElement('p');
			summary.append('No deliverables planned. ', snapshot);
			this.replaceChildren(summary);
			return;
		}
		const wrapper = document.createElement('div');
		wrapper.className = 'ex-table-scroll';
		const table = document.createElement('table');
		table.createCaption().append(this.getAttribute('label') ?? 'Deliverables', ' · ', snapshot);
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
			title.append(
				linkTo(
					source,
					item.links.plan.page,
					`${item.id} · ${item.title}`,
					item.links.plan.fragment,
				),
			);
			row.append(title);
			const summary = document.createElement('strong');
			summary.textContent = deliveryStateLabels[item.state];
			const state = row.insertCell();
			state.append(summary);
			for (const reason of item.reasons) {
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
