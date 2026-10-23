import { reportIssue } from '../core/issues.ts';
import {
	parseBreakdownValue,
	parseProgressValue,
	type BreakdownPartInput,
	type BreakdownValue,
	type ProgressValue,
} from '../core/small-parts.ts';
import { readTone } from '../core/tones.ts';

const svgNamespace = 'http://www.w3.org/2000/svg';
const progressRadius = 42;
const progressCircumference = 2 * Math.PI * progressRadius;

function textElement(document: Document, className: string, text: string): HTMLSpanElement {
	const element = document.createElement('span');
	element.className = className;
	element.textContent = text;
	return element;
}

function formatNumber(value: number): string {
	return String(value);
}

function describeProgressInvalid(
	reason: Extract<ProgressValue, { readonly kind: 'invalid' }>['reason'],
): string {
	switch (reason) {
		case 'missing-total':
			return 'a total is required when a count is supplied';
		case 'not-integer':
			return 'count and total must be whole numbers';
		case 'negative':
			return 'count and total cannot be negative';
		case 'exceeds-total':
			return 'count exceeds the total';
		default:
			throw new Error('Unknown progress validation reason.');
	}
}

/** A normal-text author label; meaning comes only from the existing optional semantic tone. */
export class BadgeIsland extends HTMLElement {
	connectedCallback(): void {
		if (this.hasAttribute('prepared')) return;
		const label = this.textContent?.trim() ?? '';
		readTone(this, undefined);
		this.replaceChildren(this.ownerDocument.createTextNode(label));
		this.setAttribute('prepared', '');
	}
}

/** A count view with exact authored text and a ring only for a valid positive denominator. */
export class ProgressIsland extends HTMLElement {
	connectedCallback(): void {
		if (this.hasAttribute('prepared')) return;
		const label = this.getAttribute('label')?.trim() ?? '';
		if (!label) reportIssue(this, '<ex-progress> needs a non-empty label.');

		const rawValue = this.getAttribute('value');
		const rawMax = this.getAttribute('max');
		const result = parseProgressValue(rawValue, rawMax);
		const content: Node[] = [textElement(this.ownerDocument, 'ex-progress-label', label)];

		if (result.kind === 'known') {
			const gauge = this.ownerDocument.createElementNS(svgNamespace, 'svg');
			gauge.classList.add('ex-progress-gauge');
			gauge.setAttribute('viewBox', '0 0 100 100');
			gauge.setAttribute('role', 'img');
			gauge.setAttribute('aria-label', `${result.value} of ${result.max} ${label}`);
			const track = this.ownerDocument.createElementNS(svgNamespace, 'circle');
			track.classList.add('ex-progress-track');
			track.setAttribute('cx', '50');
			track.setAttribute('cy', '50');
			track.setAttribute('r', String(progressRadius));
			const fill = this.ownerDocument.createElementNS(svgNamespace, 'circle');
			fill.classList.add('ex-progress-fill');
			fill.setAttribute('cx', '50');
			fill.setAttribute('cy', '50');
			fill.setAttribute('r', String(progressRadius));
			fill.setAttribute('stroke-dasharray', String(progressCircumference));
			fill.setAttribute(
				'stroke-dashoffset',
				String(progressCircumference * (1 - result.value / result.max)),
			);
			gauge.append(track);
			if (result.value > 0) gauge.append(fill);
			content.push(gauge);
			content.push(
				textElement(
					this.ownerDocument,
					'ex-progress-count',
					`${formatNumber(result.value)}/${formatNumber(result.max)}`,
				),
			);
		} else {
			let message: string;
			if (result.kind === 'unknown') message = 'Not reported';
			else if (result.kind === 'empty') message = '0/0 · empty total';
			else {
				const authored = rawMax === null ? (rawValue ?? '') : `${rawValue ?? ''}/${rawMax}`;
				const detail = describeProgressInvalid(result.reason);
				const state = result.reason === 'exceeds-total' ? 'inconsistent count' : 'invalid count';
				message = `${authored} · ${state}: ${detail}`;
				reportIssue(this, `<ex-progress> ${label}: ${authored} is an ${state}; ${detail}.`);
			}
			content.push(textElement(this.ownerDocument, 'ex-progress-fallback', message));
		}

		this.replaceChildren(...content);
		this.setAttribute('prepared', '');
	}
}

function invalidBreakdownReason(
	reason: Extract<BreakdownValue, { readonly kind: 'invalid' }>['reason'],
): string {
	switch (reason) {
		case 'missing-parts':
			return 'add one to three named phase values';
		case 'invalid-value':
			return 'phase values must be finite and non-negative';
		case 'invalid-series':
			return 'each phase needs a palette series from 1 to 3';
		case 'duplicate-phase':
			return 'phase names and palette series must be unique';
		case 'invalid-scale':
			return 'choose share or elapsed with a positive elapsed max';
		case 'exceeds-scale':
			return 'the phase total exceeds its elapsed max';
		default:
			throw new Error('Unknown breakdown validation reason.');
	}
}

function rawBreakdownText(
	label: string,
	unit: string,
	scale: string,
	max: string | null,
	parts: readonly BreakdownPartInput[],
	reason: Extract<BreakdownValue, { readonly kind: 'invalid' }>['reason'],
): string {
	const authoredParts = parts.map((part): string => {
		const value = part.value === null ? 'value not supplied' : `${part.value} ${unit}`;
		return part.label ? `${part.label.trim()}: ${value}` : value;
	});
	const authoredMax = max === null ? '' : `; max ${max} ${unit}`;
	const authoredScale = scale === 'share' ? 'share' : scale;
	return `${label}: unavailable (${invalidBreakdownReason(reason)}; ${authoredScale}${authoredMax})${authoredParts.length ? ` · ${authoredParts.join(' · ')}` : ''}`;
}

function makeBreakdownTrack(
	document: Document,
	result: Extract<ReturnType<typeof parseBreakdownValue>, { kind: 'known' }>,
	label: string,
	unit: string,
): HTMLDivElement {
	const track = document.createElement('div');
	track.className = 'ex-breakdown-track';
	track.setAttribute('role', 'img');
	track.setAttribute(
		'aria-label',
		`${label} · ${result.scale === 'share' ? 'share of total, 100 percent' : `elapsed, total ${formatNumber(result.total)} of ${formatNumber(result.max)} ${unit}`}: ${result.parts.map((part): string => `${part.label} ${formatNumber(part.value)} ${unit}`).join(', ')}`,
	);
	for (const part of result.parts) {
		const segment = document.createElement('span');
		segment.className = 'ex-breakdown-segment';
		segment.dataset['series'] = String(part.series);
		segment.setAttribute('aria-hidden', 'true');
		segment.style.width = `${(part.value / result.max) * 100}%`;
		track.append(segment);
	}
	return track;
}

function makePhaseList(
	document: Document,
	parts: readonly { readonly label: string; readonly value: number; readonly series: 1 | 2 | 3 }[],
	unit: string,
): HTMLOListElement {
	const list = document.createElement('ol');
	list.className = 'ex-breakdown-phases';
	for (const part of parts) {
		const phase = document.createElement('li');
		phase.className = 'ex-breakdown-phase';
		phase.dataset['series'] = String(part.series);
		const marker = document.createElement('span');
		marker.className = 'ex-breakdown-marker';
		marker.dataset['series'] = String(part.series);
		marker.setAttribute('aria-hidden', 'true');
		phase.append(
			marker,
			textElement(document, 'ex-breakdown-name', part.label),
			textElement(document, 'ex-breakdown-value', `${formatNumber(part.value)} ${unit}`),
		);
		list.append(phase);
	}
	return list;
}

/** A named phase observation whose parser, rather than CSS, owns every numeric decision. */
export class BreakdownIsland extends HTMLElement {
	connectedCallback(): void {
		if (this.hasAttribute('prepared')) return;

		const label = this.getAttribute('label')?.trim() ?? '';
		const unit = this.getAttribute('unit')?.trim() ?? '';
		const scale = this.getAttribute('scale');
		const max = this.getAttribute('max');
		if (!label) reportIssue(this, '<ex-breakdown> needs a non-empty label.');
		if (!unit) reportIssue(this, '<ex-breakdown> needs a non-empty unit.');

		const children = Array.from(this.children);
		const nonDataChild = children.find((child): boolean => child.localName !== 'data');
		if (nonDataChild)
			reportIssue(this, '<ex-breakdown> accepts direct <data> phase children only.');
		const inputs = children
			.filter((child): boolean => child.localName === 'data')
			.map((child): BreakdownPartInput => ({
				label: child.textContent?.trim() ?? '',
				value: child.getAttribute('value'),
				series: child.getAttribute('series'),
			}));
		const result = parseBreakdownValue(inputs, scale, max);
		// A compact share belongs to a labelled table: its row and shared legend provide context.
		// The track still carries the complete observation, scale and phase facts for assistive tools.
		const compactShare =
			this.hasAttribute('compact') && result.kind === 'known' && result.scale === 'share';
		const content: Node[] = [];
		if (!compactShare) content.push(textElement(this.ownerDocument, 'ex-breakdown-label', label));

		if (result.kind === 'invalid') {
			const fallback = rawBreakdownText(label, unit, scale ?? 'share', max, inputs, result.reason);
			reportIssue(this, `<ex-breakdown> ${fallback}.`);
			content.push(textElement(this.ownerDocument, 'ex-breakdown-unavailable', fallback));
		} else {
			const scaleLabel =
				result.kind === 'zero'
					? result.scale === 'share'
						? `Share · zero total (0 ${unit})`
						: `Elapsed · 0/${formatNumber(result.max)} ${unit}`
					: result.scale === 'share'
						? 'Share · 100% of total'
						: `Elapsed · ${formatNumber(result.total)}/${formatNumber(result.max)} ${unit}`;
			if (!compactShare)
				content.push(textElement(this.ownerDocument, 'ex-breakdown-scale', scaleLabel));
			if (result.kind === 'known')
				content.push(makeBreakdownTrack(this.ownerDocument, result, label, unit));
			if (this.hasAttribute('compact')) {
				content.push(
					textElement(
						this.ownerDocument,
						'ex-breakdown-values',
						`${result.parts.map((part): string => formatNumber(part.value)).join(' / ')} ${unit}`,
					),
				);
			} else {
				content.push(makePhaseList(this.ownerDocument, result.parts, unit));
			}
		}

		this.replaceChildren(...content);
		this.setAttribute('prepared', '');
	}
}
