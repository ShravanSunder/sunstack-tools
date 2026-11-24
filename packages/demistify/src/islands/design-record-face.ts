import { decisionDetailsId, type RecordFace } from '../core/design-record-view.ts';

export const choiceRecordFaces = new WeakMap<HTMLElement, RecordFace>();

function textNode(tag: string, text: string): HTMLElement {
	const element = document.createElement(tag);
	element.textContent = text;
	return element;
}

function sourceLink(source: {
	readonly href: string;
	readonly excerpt: string;
}): HTMLAnchorElement {
	const link = document.createElement('a');
	link.href = source.href;
	link.textContent = source.excerpt;
	return link;
}

const kindLabel = {
	need: 'requirements',
	rule: 'spec',
	realization: 'program',
	'planned-check': 'verification',
} as const;

export function recordCategoryBadge(label: string, tone?: 'done' | 'note' | 'risk'): HTMLElement {
	const badge = textNode('ex-badge', label);
	if (tone) badge.setAttribute('tone', tone);
	return badge;
}

export function recordStateLine({ record, held, ruledBy, replacement }: RecordFace): HTMLElement {
	const metadata = document.createElement('div');
	metadata.className = 'ex-record-state-line';
	if (ruledBy) metadata.append(recordCategoryBadge(kindLabel[ruledBy]));

	const facts = [record.id];
	let date: string | undefined;
	if (record.kind === 'assumption') {
		metadata.append(recordCategoryBadge(record.weight));
		if (record.status === 'open') metadata.append(recordCategoryBadge('Not confirmed', 'risk'));
		else if (record.status === 'confirmed')
			metadata.append(recordCategoryBadge('Confirmed', 'done'));
		else metadata.append(recordCategoryBadge('Refuted', 'risk'));
	} else {
		const { authority } = record;
		if (authority.state === 'owner-open') {
			if (held) facts.push(held === 1 ? '1 deliverable waits' : `${held} deliverables wait`);
		} else {
			date = authority.decidedOn;
			if (authority.state === 'agent-decided') {
				metadata.append(recordCategoryBadge(authority.weight));
				if (authority.sourceKind !== 'contract' && authority.weight === 'design')
					metadata.append(recordCategoryBadge('Not reviewed by you', 'risk'));
				facts.push(authority.by.alias);
				if (authority.sourceKind === 'contract') facts.push('from contract');
			}
		}
	}
	const plain = textNode('span', facts.join(' · '));
	if (date) {
		const time = document.createElement('time');
		time.dateTime = date;
		time.textContent = new Intl.DateTimeFormat('en-GB', {
			timeZone: 'UTC',
			day: 'numeric',
			month: 'short',
		}).format(new Date(date));
		plain.append(' · ', time);
	}
	if (replacement) plain.append(` · Replaced by ${replacement}`);
	metadata.append(plain);
	return metadata;
}

function recordReason({ record }: RecordFace): HTMLElement | undefined {
	if (record.kind === 'assumption') {
		if (record.status === 'open') return textNode('p', record.settles);
		const reason = textNode('p', record.settledBy + ' · ');
		reason.append(sourceLink(record.resolution));
		return reason;
	}
	const { authority } = record;
	if (authority.state === 'owner-open') return undefined;
	if (authority.state === 'agent-decided') {
		const reason = textNode('p', authority.rationale);
		if (authority.sourceKind === 'contract') reason.append(' · ', sourceLink(authority.source));
		return reason;
	}
	const reason = document.createElement('p');
	reason.append(sourceLink(authority.source));
	return reason;
}

export function appendRecordBasis(parent: HTMLElement, face: RecordFace): void {
	const reason = recordReason(face);
	if (reason && face.record.kind === 'choice') {
		reason.className = 'ex-record-basis';
		parent.append(reason);
	}
	let linkedWork: HTMLElement | undefined;
	if (face.dependents.length) {
		linkedWork = document.createElement('p');
		linkedWork.className = 'ex-record-dependents';
		linkedWork.append(textNode('strong', 'Rests on this'), ' ');
		for (const [index, { fragment, label }] of face.dependents.entries()) {
			if (index) linkedWork.append(' · ');
			linkedWork.append(
				fragment ? sourceLink({ href: '#' + encodeURIComponent(fragment), excerpt: label }) : label,
			);
		}
		parent.append(linkedWork);
	}
	const anchor =
		(face.record.kind === 'choice' ? reason : undefined) ??
		linkedWork ??
		parent.querySelector<HTMLElement>('legend, .ex-record-item-title');
	if (anchor) anchor.id = decisionDetailsId(parent.id);
}

export function renderRecordCard(card: HTMLElement, face: RecordFace): void {
	if (face.record.kind === 'choice') return;
	card.setAttribute('prepared', '');
	card.setAttribute('zoom', 'none');
	card.classList.add('ex-record-prose');
	const title = textNode('h4', face.record.statement);
	title.className = 'ex-record-item-title';
	const reason = recordReason(face);
	for (const child of card.querySelectorAll(':scope > [slot="details"]'))
		child.removeAttribute('slot');
	card.prepend(recordStateLine(face), title, ...(reason ? [reason] : []));
	appendRecordBasis(card, face);
}

export function inlineRecordStatus(
	element: HTMLElement,
	record: RecordFace['record'],
	href: string,
): void {
	let state: string;
	let tone: string;
	if (record.kind === 'assumption') {
		state =
			record.status === 'open'
				? 'Assumed'
				: record.status === 'confirmed'
					? 'Confirmed'
					: 'Refuted';
		tone = record.status === 'confirmed' ? 'done' : 'risk';
	} else {
		const { authority } = record;
		if (authority.state === 'owner-open') {
			state = 'Needs you';
			tone = 'question';
		} else if (authority.state === 'owner-decided') {
			state = 'You decided';
			tone = 'decision';
		} else {
			state = `${authority.by.alias} decided`;
			tone =
				authority.sourceKind === 'contract' || authority.weight !== 'design' ? 'decision' : 'risk';
		}
	}
	const link = document.createElement('a');
	link.href = href;
	const badge = textNode('ex-badge', state);
	badge.setAttribute('tone', tone);
	link.append(badge);
	element.replaceChildren(link);
}
