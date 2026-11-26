import { reportIssue } from '../core/issues.ts';
import {
	parseProjectOverviewView,
	type ProjectOverviewView,
} from '../core/project-overview-view.ts';
import { registerReviewSource, type ReviewSourceResult } from '../core/review-document.ts';
import { deliveryState, linkTo } from './deliverables.ts';
import { glossaryTerm } from './glossary.ts';

const decisionStatus = {
	Open: { tone: 'question', explanation: 'Waiting for you to decide.' },
	'Needs review': {
		tone: 'note',
		explanation: 'An agent made this decision. Work can proceed while it awaits your review.',
	},
} as const;

function appendSection(content: HTMLElement, label: string): HTMLElement {
	const section = document.createElement('section');
	const heading = document.createElement('h2');
	heading.textContent = label;
	section.append(heading);
	content.append(section);
	return section;
}

function paragraph(text: string): HTMLParagraphElement {
	const node = document.createElement('p');
	node.textContent = text;
	return node;
}

function appendList(
	section: HTMLElement,
	entries: readonly (readonly (string | Node)[])[],
	empty: string,
): void {
	if (!entries.length) {
		section.append(paragraph(empty));
		return;
	}
	const list = document.createElement('ul');
	for (const entry of entries) {
		const item = document.createElement('li');
		item.append(...entry);
		list.append(item);
	}
	section.append(list);
}

function renderDeliverables(section: HTMLElement, view: ProjectOverviewView, source: URL): void {
	const core = view.deliverables.filter((item) => item.kind === 'core');
	const later = view.deliverables.filter((item) => item.kind === 'later');
	if (core.length) {
		const progress = document.createElement('ex-progress');
		progress.setAttribute('label', 'Core deliverables accepted');
		progress.setAttribute('value', String(core.filter(({ state }) => state === 'accepted').length));
		progress.setAttribute('max', String(core.length));
		const acceptedCount = paragraph('');
		acceptedCount.append(progress);
		section.append(acceptedCount);

		const grid = document.createElement('ex-grid');
		grid.setAttribute('cols', '2');
		for (const item of core) {
			const card = document.createElement('ex-card');
			card.setAttribute('title', `${item.id.split('/').at(-1)} · ${item.title}`);
			card.setAttribute('tone', item.state === 'accepted' ? 'done' : 'note');
			const status = paragraph('');
			status.append(deliveryState(item.state));
			const plan = paragraph('');
			plan.append(
				linkTo(source, item.destination.page, 'Open deliverable', item.destination.fragment),
			);
			card.append(status, plan);
			grid.append(card);
		}
		section.append(grid);
	} else section.append(paragraph('No core deliverables planned.'));

	if (later.length) {
		const details = document.createElement('details');
		const summary = document.createElement('summary');
		summary.textContent = `Later deliverables (${later.length})`;
		details.append(summary);
		appendList(
			details,
			later.map(({ id, title }) => [`${id.split('/').at(-1)} · ${title}`]),
			'',
		);
		section.append(details);
	}
}

function appendTable(
	section: HTMLElement,
	headings: readonly string[],
	rows: readonly (readonly (string | Node)[])[],
	empty: string,
): HTMLTableElement | undefined {
	if (!rows.length) {
		section.append(paragraph(empty));
		return undefined;
	}
	const table = document.createElement('table');
	table.dataset['width'] = 'reading';
	const header = table.createTHead().insertRow();
	for (const heading of headings) {
		const column = document.createElement('th');
		column.scope = 'col';
		column.textContent = heading;
		header.append(column);
	}
	const body = table.createTBody();
	for (const cells of rows) {
		const row = body.insertRow();
		for (const cell of cells) row.insertCell().append(cell);
	}
	section.append(table);
	return table;
}

function renderDecisions(section: HTMLElement, view: ProjectOverviewView, source: URL): void {
	const open = view.decisions.filter(({ status }) => status === 'Open');
	const review = view.decisions.filter(({ status }) => status === 'Needs review');
	const table = appendTable(
		section,
		['Status', 'Decision'],
		[...open, ...review].map(({ href, title, status, outcome }) => {
			const decision = document.createDocumentFragment();
			decision.append(linkTo(source, href, title));
			if (outcome) decision.append(paragraph(outcome));
			const { tone, explanation } = decisionStatus[status];
			return [glossaryTerm(status, explanation, tone), decision];
		}),
		'No decisions need your attention.',
	);
	if (table) table.dataset['width'] = 'canvas';
}

function renderProjectOverview(
	view: ProjectOverviewView,
	source: URL,
	historyLink: HTMLAnchorElement | null,
): HTMLElement {
	const content = document.createElement('div');
	renderDeliverables(appendSection(content, 'Deliverable progress'), view, source);
	appendTable(
		appendSection(content, 'Design'),
		['Document'],
		view.documents.map(({ href, title }) => [linkTo(source, href, title)]),
		'No design documents registered.',
	);
	const decisions = appendSection(content, 'Pending decisions');
	if (historyLink) {
		const history = paragraph('');
		history.append(historyLink);
		decisions.append(history);
	}
	renderDecisions(decisions, view, source);
	appendTable(
		appendSection(content, 'Work logs'),
		['Date', 'Work log'],
		view.workLogs.map(({ href, title, date }) => {
			const time = document.createElement('time');
			time.dateTime = date;
			time.textContent = date;
			return [time, linkTo(source, href, title)];
		}),
		'No dated work logs registered.',
	);
	return content;
}

export class ProjectOverviewIsland extends HTMLElement {
	private rendered: Promise<ReviewSourceResult> | undefined;

	connectedCallback(): void {
		if (this.closest('[data-inspector-copy], [data-inspector-content]')) return;
		this.rendered ??= this.load();
		registerReviewSource(this, this.rendered);
	}

	private async load(): Promise<ReviewSourceResult> {
		this.setAttribute('aria-busy', 'true');
		try {
			const reference = this.getAttribute('src')?.trim();
			if (!reference) throw new Error('Project overview src must name generated project data.');
			const source = new URL(reference, this.baseURI);
			const response = await fetch(source);
			if (!response.ok) throw new Error(`HTTP ${response.status} loading ${reference}.`);
			const text = await response.text();
			const historyLink = this.querySelector<HTMLAnchorElement>(
				':scope > a[slot="decision-history"]',
			);
			this.replaceChildren(
				renderProjectOverview(parseProjectOverviewView(text), source, historyLink),
			);
			this.dispatchEvent(new CustomEvent('island-ready', { bubbles: true }));
			return { kind: 'loaded', source: text };
		} catch (error) {
			this.textContent = 'Project overview unavailable.';
			this.setAttribute('role', 'status');
			reportIssue(this, error instanceof Error ? error.message : 'Invalid project overview data.');
			return { kind: 'failed' };
		} finally {
			this.removeAttribute('aria-busy');
		}
	}
}
