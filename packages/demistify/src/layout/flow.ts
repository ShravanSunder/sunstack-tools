import { reportIssue } from '../core/issues.ts';
import type { Box as LabelBox } from './canvas-router.ts';
import {
	type FlowCaptionLayout,
	alignFacingFlowPorts,
	flowCaptionBounds,
	layoutFlowCaption,
	positionFlowCaption,
} from './flow-connector-layout.ts';

interface Point {
	readonly x: number;
	readonly y: number;
}
interface Box {
	readonly element: HTMLElement;
	readonly left: number;
	readonly right: number;
	readonly top: number;
	readonly bottom: number;
	row: number;
	column: number;
}
interface Band {
	start: number;
	end: number;
}

type Side = 'top' | 'bottom' | 'left' | 'right';
interface Port {
	readonly key: number;
	position: number;
}
interface Route {
	readonly arrow: Element;
	readonly source: Box;
	readonly target: Box;
	readonly exit: { readonly side: Side; readonly port: Port };
	readonly entry: { readonly side: Side; readonly port: Port };
}

const svgNamespace = 'http://www.w3.org/2000/svg';
const laneSpacing = 8;
const cornerRadius = 8;

/** Groups intervals that overlap into ordered bands (rows or columns of a layout). */
function bandsOf(intervals: readonly Band[]): Band[] {
	const sorted = intervals.toSorted((first, second): number => first.start - second.start);
	const bands: Band[] = [];
	for (const interval of sorted) {
		const last = bands.at(-1);
		if (last && interval.start < last.end - 2) last.end = Math.max(last.end, interval.end);
		else bands.push({ ...interval });
	}
	return bands;
}

function bandIndex(bands: readonly Band[], start: number): number {
	return bands.findIndex((band): boolean => start >= band.start - 2 && start < band.end);
}

function isVertical(side: Side): boolean {
	return side === 'top' || side === 'bottom';
}

function centerOf(box: Box): Point {
	return { x: (box.left + box.right) / 2, y: (box.top + box.bottom) / 2 };
}

function sidePoint(box: Box, side: Side, position: number): Point {
	if (side === 'top') return { x: position, y: box.top };
	if (side === 'bottom') return { x: position, y: box.bottom };
	return { x: side === 'left' ? box.left : box.right, y: position };
}

/** Which side an arrow leaves and enters, so that it only ever travels through empty channels. */
function planSides(source: Box, target: Box): { readonly exit: Side; readonly entry: Side } {
	if (source.row === target.row && Math.abs(source.column - target.column) === 1)
		return target.column > source.column
			? { exit: 'right', entry: 'left' }
			: { exit: 'left', entry: 'right' };
	if (source.row === target.row) return { exit: 'top', entry: 'top' };
	return target.row > source.row
		? { exit: 'bottom', entry: 'top' }
		: { exit: 'top', entry: 'bottom' };
}

function roundedPath(points: readonly Point[]): string {
	const first = points[0];
	if (!first) return '';
	let path = `M${first.x},${first.y}`;
	for (let index = 1; index < points.length - 1; index += 1) {
		const previous = points[index - 1];
		const corner = points[index];
		const next = points[index + 1];
		if (!previous || !corner || !next) continue;
		const inLength = Math.hypot(corner.x - previous.x, corner.y - previous.y);
		const outLength = Math.hypot(next.x - corner.x, next.y - corner.y);
		const radius = Math.min(cornerRadius, inLength / 2, outLength / 2);
		const before = {
			x: corner.x - ((corner.x - previous.x) / (inLength || 1)) * radius,
			y: corner.y - ((corner.y - previous.y) / (inLength || 1)) * radius,
		};
		const after = {
			x: corner.x + ((next.x - corner.x) / (outLength || 1)) * radius,
			y: corner.y + ((next.y - corner.y) / (outLength || 1)) * radius,
		};
		path += ` L${before.x},${before.y} Q${corner.x},${corner.y} ${after.x},${after.y}`;
	}
	const last = points.at(-1);
	return last ? `${path} L${last.x},${last.y}` : path;
}

/**
 * Lays islands out in a grid and routes its arrows only through the empty channels between rows
 * and columns, so an arrow never crosses a box. Parallel arrows in one channel get separate lanes.
 */
export class FlowIsland extends HTMLElement {
	private overlay: SVGSVGElement | undefined;
	private resizeObserver: ResizeObserver | undefined;
	private frame: number | undefined;
	private readonly schedule = (): void => {
		this.frame ??= requestAnimationFrame((): void => {
			this.frame = undefined;
			this.route();
		});
	};

	connectedCallback(): void {
		if (this.hasAttribute('data-inspector-content')) return;
		if (!this.overlay) {
			const overlay =
				this.querySelector<SVGSVGElement>(':scope > .ex-flow-overlay') ??
				document.createElementNS(svgNamespace, 'svg');
			overlay.classList.add('ex-flow-overlay');
			overlay.setAttribute('aria-hidden', 'true');
			const markerId = `ex-flow-${crypto.randomUUID()}`;
			overlay.innerHTML = `<defs><marker id="${markerId}" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0,0 L8,4 L0,8 Z" fill="currentColor"/></marker></defs>`;
			overlay.dataset['marker'] = markerId;
			this.prepend(overlay);
			this.overlay = overlay;
		}
		// Observers are rebuilt on every connection, so a flow moved elsewhere keeps routing.
		this.resizeObserver?.disconnect();
		this.resizeObserver = new ResizeObserver(this.schedule);
		this.resizeObserver.observe(this);
		for (const child of this.islands()) this.resizeObserver.observe(child);
		this.addEventListener('island-ready', this.schedule);
		document.fonts.addEventListener('loadingdone', this.schedule);
		this.schedule();
	}

	disconnectedCallback(): void {
		this.resizeObserver?.disconnect();
		document.fonts.removeEventListener('loadingdone', this.schedule);
		if (this.frame !== undefined) cancelAnimationFrame(this.frame);
		this.frame = undefined;
	}

	private islands(): HTMLElement[] {
		return Array.from(this.children).filter(
			(child): child is HTMLElement =>
				child instanceof HTMLElement && child.localName !== 'ex-arrow' && !child.hidden,
		);
	}

	private route(): void {
		const overlay = this.overlay;
		if (!overlay || !this.checkVisibility()) return;
		const captionLayouts = this.prepareCaptions(overlay);
		// Work in the flow's own units so routing stays correct when a camera or zoom scales it.
		const screen = this.getBoundingClientRect();
		const scale = this.offsetWidth > 0 ? screen.width / this.offsetWidth : 1;
		const origin = {
			left: screen.left,
			top: screen.top,
			width: this.offsetWidth,
			height: this.offsetHeight,
		};
		// The overlay is sized by CSS to exactly the flow and draws in the flow's own pixels, so a
		// stale size can never stick out and make a scrolling parent flip its scrollbars.
		const boxes = new Map<string, Box>();
		for (const element of this.islands()) {
			const bounds = element.getBoundingClientRect();
			if (!element.id || bounds.width === 0) continue;
			boxes.set(element.id, {
				element,
				left: (bounds.left - origin.left) / scale,
				right: (bounds.right - origin.left) / scale,
				top: (bounds.top - origin.top) / scale,
				bottom: (bounds.bottom - origin.top) / scale,
				row: 0,
				column: 0,
			});
		}
		const rows = bandsOf(
			Array.from(boxes.values(), (box): Band => ({ start: box.top, end: box.bottom })),
		);
		const columns = bandsOf(
			Array.from(boxes.values(), (box): Band => ({ start: box.left, end: box.right })),
		);
		for (const box of boxes.values()) {
			box.row = bandIndex(rows, box.top);
			box.column = bandIndex(columns, box.left);
		}
		const laneUse = new Map<string, number>();
		const lane = (channel: string): number => {
			const used = laneUse.get(channel) ?? 0;
			laneUse.set(channel, used + 1);
			return (used % 2 === 0 ? 1 : -1) * Math.ceil(used / 2) * laneSpacing;
		};
		const boundaryY = (index: number): number => {
			const above = rows[index - 1];
			const below = rows[index];
			if (above && below) return (above.end + below.start) / 2;
			return below ? below.start / 2 : ((above?.end ?? 0) + origin.height) / 2;
		};
		const boundaryX = (index: number): number => {
			const before = columns[index - 1];
			const after = columns[index];
			if (before && after) return (before.end + after.start) / 2;
			return after ? after.start / 2 : ((before?.end ?? 0) + origin.width) / 2;
		};
		const plans = Array.from(this.querySelectorAll(':scope > ex-arrow'), (arrow) => {
			const source = boxes.get(arrow.getAttribute('from') ?? '');
			const target = boxes.get(arrow.getAttribute('to') ?? '');
			return source && target ? { arrow, source, target, ...planSides(source, target) } : undefined;
		}).filter((plan) => plan !== undefined);
		const portGroups = new Map<
			string,
			{ readonly box: Box; readonly side: Side; readonly ports: Port[] }
		>();
		const claimPort = (box: Box, side: Side, key: number): Port => {
			const groupKey = `${box.element.id}|${side}`;
			const group = portGroups.get(groupKey) ?? { box, side, ports: [] };
			const port: Port = { key, position: 0 };
			group.ports.push(port);
			portGroups.set(groupKey, group);
			return port;
		};
		const routes = plans.map((plan): Route => {
			const targetCenter = centerOf(plan.target);
			const sourceCenter = centerOf(plan.source);
			return {
				arrow: plan.arrow,
				source: plan.source,
				target: plan.target,
				exit: {
					side: plan.exit,
					port: claimPort(
						plan.source,
						plan.exit,
						isVertical(plan.exit) ? targetCenter.x : targetCenter.y,
					),
				},
				entry: {
					side: plan.entry,
					port: claimPort(
						plan.target,
						plan.entry,
						isVertical(plan.entry) ? sourceCenter.x : sourceCenter.y,
					),
				},
			};
		});
		for (const { box, side, ports } of portGroups.values()) {
			const [low, high] = isVertical(side) ? [box.left, box.right] : [box.top, box.bottom];
			const inset = (high - low) * 0.2;
			ports.sort((first, second): number => first.key - second.key);
			ports.forEach((port, index): void => {
				port.position = low + inset + ((high - low - 2 * inset) * (index + 1)) / (ports.length + 1);
			});
		}
		const fragment = document.createDocumentFragment();
		const routedPoints: Point[][] = [];
		const captions: {
			readonly arrow: Element;
			readonly element: SVGTextElement;
			readonly background: SVGRectElement;
			readonly layout: FlowCaptionLayout;
			readonly points: readonly Point[];
			readonly routeIndex: number;
		}[] = [];
		for (const [routeIndex, route] of routes.entries()) {
			const start = sidePoint(route.source, route.exit.side, route.exit.port.position);
			const end = sidePoint(route.target, route.entry.side, route.entry.port.position);
			const points = this.connect(route, start, end, boundaryY, boundaryX, lane);
			routedPoints.push(points);
			const path = document.createElementNS(svgNamespace, 'path');
			path.classList.add('ex-arrow-line');
			path.setAttribute('d', roundedPath(points));
			path.setAttribute('marker-end', `url(#${overlay.dataset['marker'] ?? ''})`);
			const ends = {
				from: route.arrow.getAttribute('from') ?? '',
				to: route.arrow.getAttribute('to') ?? '',
			};
			Object.assign(path.dataset, ends);
			fragment.append(path);
			const labelText = route.arrow.getAttribute('label');
			const layout = captionLayouts.get(route.arrow);
			if (labelText && layout) {
				const label = document.createElementNS(svgNamespace, 'text');
				label.classList.add('ex-arrow-label');
				label.setAttribute('dominant-baseline', 'central');
				Object.assign(label.dataset, ends);
				const background = document.createElementNS(svgNamespace, 'rect');
				background.classList.add('ex-flow-label-background');
				fragment.append(background, label);
				captions.push({
					arrow: route.arrow,
					element: label,
					background,
					layout,
					points,
					routeIndex,
				});
			}
		}
		overlay.querySelectorAll(':scope > :not(defs)').forEach((node): void => node.remove());
		overlay.append(fragment);
		const occupied = Array.from(boxes.values());
		const placed: LabelBox[] = [];
		const lineBounds = routedPoints.map((points): LabelBox[] =>
			points.slice(1).map((after, index): LabelBox => {
				const before = points[index] ?? after;
				return {
					left: Math.min(before.x, after.x) - 1,
					right: Math.max(before.x, after.x) + 1,
					top: Math.min(before.y, after.y) - 1,
					bottom: Math.max(before.y, after.y) + 1,
				};
			}),
		);
		for (const caption of captions) {
			const position = positionFlowCaption({
				points: caption.points,
				width: caption.layout.width,
				height: caption.layout.height,
				bounds: { left: 0, top: 0, right: origin.width, bottom: origin.height },
				obstacles: [
					...occupied,
					...lineBounds.filter((_, index): boolean => index !== caption.routeIndex).flat(),
				],
				placed,
			});
			if (!position) {
				caption.element.remove();
				caption.background.remove();
				reportIssue(
					caption.arrow,
					`No clear space for arrow label "${caption.arrow.getAttribute('label') ?? ''}" after wrapping. Leave more room around the connected cards.`,
				);
				continue;
			}
			const bounds = flowCaptionBounds(position, caption.layout.width, caption.layout.height);
			caption.background.setAttribute('x', String(bounds.left));
			caption.background.setAttribute('y', String(bounds.top));
			caption.background.setAttribute('width', String(bounds.right - bounds.left));
			caption.background.setAttribute('height', String(bounds.bottom - bounds.top));
			for (const [index, line] of caption.layout.lines.entries()) {
				const span = document.createElementNS(svgNamespace, 'tspan');
				span.setAttribute('x', String(position.x));
				span.setAttribute(
					'y',
					String(
						position.y +
							(index - (caption.layout.lines.length - 1) / 2) * caption.layout.lineHeight,
					),
				);
				span.textContent = line;
				caption.element.append(span);
			}
			placed.push(bounds);
		}
	}

	private prepareCaptions(overlay: SVGSVGElement): ReadonlyMap<Element, FlowCaptionLayout> {
		const measurement = document.createElementNS(svgNamespace, 'text');
		measurement.classList.add('ex-arrow-label');
		measurement.setAttribute('visibility', 'hidden');
		overlay.append(measurement);
		const measureText = (text: string): number => {
			measurement.textContent = text;
			return measurement.getComputedTextLength();
		};
		const lineHeight = Math.ceil(
			(Number.parseFloat(getComputedStyle(measurement).fontSize) || 14) * 1.35,
		);
		const captions = new Map<Element, FlowCaptionLayout>();
		for (const arrow of this.querySelectorAll(':scope > ex-arrow')) {
			const text = arrow.getAttribute('label')?.trim();
			if (text) captions.set(arrow, layoutFlowCaption({ text, measureText, lineHeight }));
		}
		measurement.remove();
		if (captions.size > 0) {
			const layouts = [...captions.values()];
			this.style.setProperty(
				'--flow-label-column-gap',
				`${Math.max(...layouts.map((layout): number => layout.columnGap))}px`,
			);
			this.style.setProperty(
				'--flow-label-row-gap',
				`${Math.max(...layouts.map((layout): number => layout.rowGap))}px`,
			);
		} else {
			this.style.removeProperty('--flow-label-column-gap');
			this.style.removeProperty('--flow-label-row-gap');
		}
		return captions;
	}

	private connect(
		route: Route,
		start: Point,
		end: Point,
		boundaryY: (index: number) => number,
		boundaryX: (index: number) => number,
		lane: (channel: string) => number,
	): Point[] {
		const { source, target } = route;
		if (route.exit.side === 'right' || route.exit.side === 'left') {
			const aligned = alignFacingFlowPorts({
				source,
				target,
				start,
				end,
				orientation: 'horizontal',
			});
			if (aligned) return [...aligned];
			const y = Math.abs(start.y - end.y) < 24 ? (start.y + end.y) / 2 : start.y;
			const middleX = (start.x + end.x) / 2;
			return y === start.y && Math.abs(start.y - end.y) >= 24
				? [start, { x: middleX, y: start.y }, { x: middleX, y: end.y }, end]
				: [
						{ x: start.x, y },
						{ x: end.x, y },
					];
		}
		if (source.column === target.column && Math.abs(source.row - target.row) === 1) {
			const aligned = alignFacingFlowPorts({ source, target, start, end, orientation: 'vertical' });
			if (aligned) return [...aligned];
			return [
				start,
				{ x: start.x, y: (start.y + end.y) / 2 },
				{ x: end.x, y: (start.y + end.y) / 2 },
				end,
			];
		}
		const exitBoundary = route.exit.side === 'bottom' ? source.row + 1 : source.row;
		const entryBoundary = route.entry.side === 'top' ? target.row : target.row + 1;
		const exitY = boundaryY(exitBoundary) + lane(`row-${exitBoundary}`);
		if (exitBoundary === entryBoundary)
			return [start, { x: start.x, y: exitY }, { x: end.x, y: exitY }, end];
		const entryY = boundaryY(entryBoundary) + lane(`row-${entryBoundary}`);
		const columnBoundary = target.column >= source.column ? target.column : target.column + 1;
		const channelX = boundaryX(columnBoundary) + lane(`column-${columnBoundary}`);
		return [
			start,
			{ x: start.x, y: exitY },
			{ x: channelX, y: exitY },
			{ x: channelX, y: entryY },
			{ x: end.x, y: entryY },
			end,
		];
	}
}
