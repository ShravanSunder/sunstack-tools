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

const recordKindBadges = {
	need: { label: 'requirements', icon: 'target' },
	rule: { label: 'spec', icon: 'article' },
	realization: { label: 'program', icon: 'code' },
	'planned-check': { label: 'verification', icon: 'shield-check' },
} as const;

export function recordCategoryBadge(
	label: string,
	tone?: 'done' | 'note' | 'risk' | 'question' | 'decision',
): HTMLElement {
	const badge = textNode('ex-badge', label);
	if (tone) badge.setAttribute('tone', tone);
	return badge;
}

function recordPresentation(record: RecordFace['record']): {
	readonly label: string;
	readonly metadataLabel?: string;
	readonly tone: 'done' | 'risk' | 'question' | 'decision';
	readonly reviewPending: boolean;
} {
	if (record.kind === 'assumption') {
		if (record.status === 'confirmed')
			return { label: 'Confirmed', tone: 'done', reviewPending: false };
		if (record.status === 'open')
			return {
				label: 'Assumed',
				metadataLabel: 'Not confirmed',
				tone: 'risk',
				reviewPending: false,
			};
		return { label: 'Refuted', tone: 'risk', reviewPending: false };
	}
	const { authority } = record;
	if (authority.state === 'owner-open')
		return { label: 'Needs you', tone: 'question', reviewPending: false };
	if (authority.state === 'owner-decided')
		return { label: 'You decided', tone: 'decision', reviewPending: false };
	const reviewPending = authority.sourceKind !== 'contract' && authority.weight === 'design';
	return {
		label: `${authority.by.alias} decided`,
		tone: reviewPending ? 'risk' : 'decision',
		reviewPending,
	};
}

export function recordStateLine({ record, held, ruledBy, replacement }: RecordFace): HTMLElement {
	const metadata = document.createElement('div');
	metadata.className = 'ex-record-state-line';
	if (ruledBy) {
		const { label, icon } = recordKindBadges[ruledBy];
		const badge = recordCategoryBadge(label);
		badge.setAttribute('icon', icon);
		metadata.append(badge);
	}
	const presentation = recordPresentation(record);

	const facts = [record.id];
	let date: string | undefined;
	if (record.kind === 'assumption') {
		metadata.append(recordCategoryBadge(record.weight));
		metadata.append(
			recordCategoryBadge(presentation.metadataLabel ?? presentation.label, presentation.tone),
		);
	} else {
		const { authority } = record;
		if (authority.state === 'owner-open') {
			if (held) facts.push(held === 1 ? '1 deliverable waits' : `${held} deliverables wait`);
		} else {
			date = authority.decidedOn;
			if (authority.state === 'agent-decided') {
				metadata.append(recordCategoryBadge(authority.weight));
				if (presentation.reviewPending)
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

function recordReason({ record }: RecordFace, chosenTitle?: string): HTMLElement | undefined {
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

	const label = authority.sourceKind === 'owner-words' ? 'Your words' : 'Wording you accepted';
	reason.append(
		textNode('strong', label),
		' ',
		sourceLink({
			href: authority.source.href,
			excerpt: chosenTitle === authority.source.excerpt ? 'your answer' : authority.source.excerpt,
		}),
	);
	return reason;
}

export function appendRecordBasis(
	parent: HTMLElement,
	face: RecordFace,
	{
		body = parent,
		chosenTitle,
	}: { readonly body?: HTMLElement; readonly chosenTitle?: string } = {},
): void {
	const reason = recordReason(face, chosenTitle);
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
		body.append(linkedWork);
	}
	const anchor =
		(face.record.kind === 'choice' ? reason : undefined) ??
		linkedWork ??
		body.querySelector<HTMLElement>('legend, .ex-record-item-title');
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

export function renderSectionRecord(item: HTMLElement, face: RecordFace): void {
	if (face.record.kind === 'choice') return;
	const title = textNode('h4', face.record.statement);
	title.className = 'ex-record-item-title';
	item.dataset['commentLabel'] = face.record.statement;
	const body = document.createElement('article');
	body.className = 'ex-record-surface';
	body.dataset['commentSlot'] = '';
	const reason = recordReason(face);
	body.append(title, recordStateLine(face), ...(reason ? [reason] : []), ...item.childNodes);
	item.append(body);
	appendRecordBasis(item, face, { body });
}

export function inlineRecordStatus(
	element: HTMLElement,
	record: RecordFace['record'],
	href: string,
): void {
	const { label, tone } = recordPresentation(record);
	const link = document.createElement('a');
	link.href = href;
	const badge = recordCategoryBadge(label, tone);
	link.append(badge);
	element.replaceChildren(link);
}
