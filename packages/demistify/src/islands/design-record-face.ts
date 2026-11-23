import { decisionDetailsId, type RecordFace } from '../core/design-record-view.ts';
import { createHead } from '../core/head.ts';
import { ChoiceDetailPanel } from './choice-detail-panel.ts';

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

interface StateLine {
	readonly state: string;
	readonly icon: string;
	readonly tone: string;
	readonly basis?: string;
	readonly date?: string;
	readonly weight?: string;
}

function recordState({ record, replacement }: RecordFace): StateLine {
	if (replacement) return { state: `Replaced by ${replacement}`, icon: 'file', tone: 'removed' };
	if (record.kind === 'assumption') {
		const weight = record.weight === 'design' ? undefined : record.weight;
		if (record.status === 'open')
			return {
				state: 'Assumed',
				basis: 'not confirmed',
				icon: 'robot',
				tone: weight ? 'note' : 'risk',
				...(weight && { weight }),
			};
		return {
			state: record.status === 'confirmed' ? 'Confirmed' : 'Refuted',
			icon: 'robot',
			tone: record.status === 'confirmed' ? 'done' : 'removed',
			...(weight && { weight }),
		};
	}
	const { authority } = record;
	if (authority.state === 'owner-open')
		return { state: 'Needs you', icon: 'user', tone: 'question' };
	if (authority.state === 'owner-decided')
		return {
			state: 'You decided',
			icon: authority.sourceKind === 'owner-words' ? 'user' : 'robot',
			tone: 'decision',
			date: authority.decidedOn,
			basis: authority.sourceKind === 'owner-words' ? 'your words' : 'agent’s wording you accepted',
		};
	const isContract = authority.sourceKind === 'contract';
	return {
		state: `${authority.by.alias} decided`,
		icon: isContract ? 'file' : 'robot',
		basis: isContract ? 'from contract' : 'own call',
		tone: isContract || authority.weight !== 'design' ? 'decision' : 'risk',
		date: authority.decidedOn,
		...(authority.weight !== 'design' && { weight: authority.weight }),
	};
}

const kindLabel = {
	need: 'requirements',
	rule: 'spec',
	realization: 'program',
	'planned-check': 'verification',
} as const;

export function recordStateLine(
	face: RecordFace,
	placement: string,
	beside: ChoiceDetailPanel,
): HTMLElement {
	const { state, icon, tone, basis, date, weight } = recordState(face);
	const row = createHead('div', 'ex-record-state-line', { title: state, icon, tone: undefined });
	row.dataset['recordTone'] = tone;
	if (basis) row.append(textNode('span', basis));
	if (state === 'Needs you' && face.held)
		row.append(
			textNode('small', face.held === 1 ? '1 deliverable waits' : `${face.held} deliverables wait`),
		);
	if (date) {
		const dateLabel = new Intl.DateTimeFormat('en-GB', {
			timeZone: 'UTC',
			day: 'numeric',
			month: 'short',
		}).format(new Date(date));
		const time = document.createElement('time');
		time.dateTime = date;
		time.textContent = dateLabel;
		row.append(time);
	}
	if (weight) {
		const chip = textNode('span', weight);
		chip.className = 'chip ex-head-tag';
		row.append(chip);
	}
	if (face.ruledBy) {
		const kind = textNode('span', kindLabel[face.ruledBy]);
		kind.className = 'ex-record-governs';
		row.append(kind);
	}
	const detail = document.createElement('div');
	const back = document.createElement('a');
	back.href = '#' + encodeURIComponent(placement);
	back.textContent = 'Back to the record';
	detail.append(back);
	appendRecordBasis(detail, face);
	const trigger = beside.add(detail, face.record.title ?? placement, decisionDetailsId(placement));
	trigger.classList.add('ex-choice-details-button', 'ex-record-detail-button');
	row.append(trigger);
	return row;
}

function recordBasis(face: RecordFace): HTMLElement | undefined {
	const { record } = face;
	const basis = document.createElement('p');
	basis.className = 'ex-record-basis';
	basis.append(textNode('strong', 'Basis'), ' ');
	if (record.kind === 'assumption') {
		if (record.status === 'open') basis.append(record.by + ' · ' + record.settles);
		else basis.append(record.settledBy + ' · ', sourceLink(record.resolution));
		return basis;
	}
	if (record.authority.state === 'owner-open') return undefined;
	if ('source' in record.authority) basis.append(sourceLink(record.authority.source));
	if (record.authority.state === 'agent-decided') {
		if ('source' in record.authority) basis.append(' · ');
		basis.append(record.authority.rationale);
	}
	return basis;
}

export function appendRecordBasis(parent: HTMLElement, face: RecordFace): void {
	const basis = recordBasis(face);
	if (basis) parent.append(basis);
	if (face.dependents.length) {
		const line = textNode('p', '');
		line.className = 'ex-record-dependents';
		line.append(
			textNode('strong', 'Rests on this'),
			' ',
			face.dependents.map(({ label }) => label).join(' · '),
		);
		parent.append(line);
	}
}

export function renderRecordCard(card: HTMLElement, face: RecordFace): void {
	if (face.record.kind === 'choice') return;
	const beside = new ChoiceDetailPanel();
	const row = recordStateLine(face, card.id, beside);
	const state = recordState(face);
	card.setAttribute('tone', state.tone);
	card.setAttribute('prepared', '');
	card.setAttribute('zoom', 'none');
	const statement = textNode('h4', face.record.statement);
	statement.className = 'ex-record-statement';
	card.prepend(row, statement);
	appendRecordBasis(card, face);
	card.before(beside.layout);
	beside.options.append(card);
	beside.layout.classList.add('ex-record-card-layout', 'ex-choice-beside-fieldset');
}

export function inlineRecordStatus(
	element: HTMLElement,
	record: RecordFace['record'],
	href: string,
): void {
	const { state, tone } = recordState({ record, dependents: [] });
	const link = document.createElement('a');
	link.href = href;
	const badge = textNode('ex-badge', state);
	badge.setAttribute('tone', tone);
	link.append(badge);
	element.replaceChildren(link);
}
