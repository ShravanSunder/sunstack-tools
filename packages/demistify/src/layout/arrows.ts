import { reportIssue } from '../core/issues.ts';
import {
	type Box,
	type Point,
	type Side,
	placeLabel,
	roundedPath,
	routeConnector,
	separateLanes,
	sides,
} from './canvas-router.ts';

/** Elements an arrow must route around. Arrow endpoints are always obstacles too. */
const obstacleSelector =
	'ex-card, ex-callout, ex-markdown, ex-mermaid, ex-file-tree, ex-zoom, ex-stat, ex-code, ex-decision, ex-compare, ex-timeline, ex-layer, ex-tabs, ex-flow, ex-region, img, figure, table, pre';
const inspectionScope = '.ex-drawer, dialog';

const activeArrows = new Set<ArrowIsland>();
const watchedContainers = new WeakSet<Element>();
let redrawFrame: number | undefined;
let canvasCount = 0;

function scheduleArrows(): void {
	if (redrawFrame !== undefined) return;
	redrawFrame = requestAnimationFrame((): void => {
		redrawFrame = undefined;
		drawAll();
	});
}

const resizeWatcher = new ResizeObserver(scheduleArrows);
const mutationWatcher = new MutationObserver((records): void => {
	if (records.some((record): boolean => !(record.target instanceof SVGElement))) scheduleArrows();
});

function endpoints(arrow: ArrowIsland): [HTMLElement | null, HTMLElement | null] {
	return [
		document.getElementById(arrow.getAttribute('from') ?? ''),
		document.getElementById(arrow.getAttribute('to') ?? ''),
	];
}

/** The nearest element holding both ends; arrows draw inside it so they scroll, clip and zoom with it. */
function containerFor(source: Element, target: Element): HTMLElement {
	let container: Element | null = source.parentElement;
	while (container && !container.contains(target)) container = container.parentElement;
	const holder = container instanceof HTMLElement ? container : document.body;
	if (holder !== document.body && getComputedStyle(holder).position === 'static')
		holder.style.position = 'relative';
	return holder;
}

function allowedSides(value: string | null): readonly Side[] {
	return value === 'left' || value === 'right' || value === 'top' || value === 'bottom'
		? [value]
		: sides;
}

/** Draws every free arrow, routing the arrows that share a container together. */
function drawAll(): void {
	const groups = new Map<HTMLElement, ArrowIsland[]>();
	for (const arrow of activeArrows) {
		const [source, target] = endpoints(arrow);
		const shown =
			source?.checkVisibility() &&
			target?.checkVisibility() &&
			source.closest(inspectionScope) === target.closest(inspectionScope);
		if (!source || !target || !shown) {
			arrow.hide();
			continue;
		}
		const container = containerFor(source, target);
		groups.set(container, [...(groups.get(container) ?? []), arrow]);
	}
	for (const [container, arrows] of groups) drawContainer(container, arrows);
}

function drawContainer(container: HTMLElement, arrows: readonly ArrowIsland[]): void {
	if (!watchedContainers.has(container)) {
		watchedContainers.add(container);
		resizeWatcher.observe(container);
		mutationWatcher.observe(container, {
			subtree: true,
			childList: true,
			attributes: true,
			attributeFilter: ['style', 'class', 'hidden', 'open'],
		});
	}
	const frame = container.getBoundingClientRect();
	const scale = container.offsetWidth > 0 ? frame.width / container.offsetWidth : 1;
	const local = (element: Element): Box => {
		const bounds = element.getBoundingClientRect();
		const left = (bounds.left - frame.left) / scale - container.clientLeft + container.scrollLeft;
		const top = (bounds.top - frame.top) / scale - container.clientTop + container.scrollTop;
		return { left, top, right: left + bounds.width / scale, bottom: top + bounds.height / scale };
	};
	const ends = arrows.map(endpoints);
	const endElements = new Set(ends.flat().filter((end): end is HTMLElement => end !== null));
	const candidates = [
		...new Set([...container.querySelectorAll(obstacleSelector), ...endElements]),
	].filter(
		(element): boolean =>
			element.checkVisibility() &&
			!element.closest('.ex-arrow-canvas') &&
			element.getBoundingClientRect().width > 0,
	);
	const routes: (Point[] | undefined)[] = [];
	arrows.forEach((arrow, index): void => {
		const [source, target] = ends[index] ?? [null, null];
		if (!source || !target) {
			routes.push(undefined);
			return;
		}
		// Outermost boxes only; never either end, a box holding an end, or content inside an end.
		const kept = candidates.filter(
			(element): boolean =>
				!element.contains(source) &&
				!element.contains(target) &&
				!source.contains(element) &&
				!target.contains(element),
		);
		const obstacles = kept
			.filter(
				(element): boolean =>
					!kept.some((other): boolean => other !== element && other.contains(element)),
			)
			.map(local);
		routes.push(
			routeConnector(
				{
					source: local(source),
					target: local(target),
					sourceSides: allowedSides(arrow.getAttribute('from-anchor')),
					targetSides: allowedSides(arrow.getAttribute('to-anchor')),
				},
				obstacles,
				routes,
			),
		);
	});
	separateLanes(routes);
	const boxes = candidates.map(local);
	// Labels also keep off every other arrow's line.
	const lines = routes.map((route): Box[] =>
		(route ?? []).slice(1).map((point, index): Box => {
			const before = route?.[index] ?? point;
			return {
				left: Math.min(before.x, point.x) - 1,
				right: Math.max(before.x, point.x) + 1,
				top: Math.min(before.y, point.y) - 1,
				bottom: Math.max(before.y, point.y) + 1,
			};
		}),
	);
	const placedLabels: Box[] = [];
	arrows.forEach((arrow, index): void => {
		const [source, target] = ends[index] ?? [null, null];
		if (source && target)
			arrow.draw(
				container,
				routes[index],
				local(source),
				local(target),
				[...boxes, ...lines.filter((_, other): boolean => other !== index).flat()],
				placedLabels,
			);
	});
}

/**
 * A labelled arrow between two islands placed by the author (grid, flex or absolute layout). It
 * routes around every other island and keeps its label clear of boxes and other labels.
 */
export class ArrowIsland extends HTMLElement {
	private group: SVGGElement | undefined;
	private routed = true;
	private length = Infinity;
	connectedCallback(): void {
		if (
			this.parentElement?.localName === 'ex-flow' ||
			this.parentElement?.localName === 'ex-region'
		) {
			window.setTimeout((): void => this.verifyEndpoints(), 2000);
			return;
		}
		activeArrows.add(this);
		for (const end of endpoints(this)) if (end) resizeWatcher.observe(end);
		if (activeArrows.size === 1) {
			window.addEventListener('resize', scheduleArrows, { passive: true });
			document.addEventListener('island-ready', scheduleArrows);
		}
		scheduleArrows();
		window.setTimeout((): void => this.verifyConnection(), 2000);
	}
	disconnectedCallback(): void {
		activeArrows.delete(this);
		this.hide();
		if (activeArrows.size === 0) {
			window.removeEventListener('resize', scheduleArrows);
			document.removeEventListener('island-ready', scheduleArrows);
		}
		scheduleArrows();
	}
	hide(): void {
		this.group?.remove();
	}
	private verifyEndpoints(): void {
		for (const name of ['from', 'to']) {
			const endpointId = this.getAttribute(name);
			if (!endpointId || !document.getElementById(endpointId))
				reportIssue(this, `${name}="${endpointId ?? ''}" does not match any element id.`);
		}
	}
	private verifyConnection(): void {
		this.verifyEndpoints();
		if (!this.group?.isConnected) return;
		const label = `${this.getAttribute('from') ?? ''} → ${this.getAttribute('to') ?? ''}`;
		if (!this.routed)
			reportIssue(
				this,
				`Arrow ${label} has no clear path between the boxes; leave a gap of at least 28px around the boxes it must pass.`,
			);
		else if (this.length < 28)
			reportIssue(
				this,
				`Arrow ${label} is only ${Math.round(this.length)}px long and hard to see; leave a wider gap between the two boxes.`,
			);
	}
	draw(
		container: HTMLElement,
		route: Point[] | undefined,
		source: Box,
		target: Box,
		boxes: readonly Box[],
		placedLabels: Box[],
	): void {
		let canvas = container.querySelector<SVGSVGElement>(':scope > .ex-arrow-canvas');
		if (!canvas) {
			canvas = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
			canvas.classList.add('ex-arrow-canvas');
			canvas.setAttribute('aria-hidden', 'true');
			canvasCount += 1;
			canvas.dataset['head'] = `ex-arrow-head-${canvasCount}`;
			canvas.innerHTML = `<defs><marker id="${canvas.dataset['head']}" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0,0 L8,4 L0,8 Z" fill="currentColor"/></marker></defs>`;
			container.append(canvas);
		}
		if (!this.group || this.group.ownerSVGElement !== canvas) {
			this.group?.remove();
			this.group = document.createElementNS('http://www.w3.org/2000/svg', 'g');
			this.group.innerHTML = `<path class="ex-arrow-line" marker-end="url(#${canvas.dataset['head'] ?? ''})"/><text class="ex-arrow-label" dominant-baseline="central"/>`;
			canvas.append(this.group);
		}
		const path = this.group.querySelector('path');
		const caption = this.group.querySelector('text');
		if (!path || !caption) return;
		const points = route ?? [
			{ x: source.right, y: (source.top + source.bottom) / 2 },
			{ x: target.left, y: (target.top + target.bottom) / 2 },
		];
		path.setAttribute('d', roundedPath(points));
		path.classList.toggle('ex-arrow-blocked', !route);
		caption.textContent = this.getAttribute('label') ?? '';
		const width = caption.textContent
			? caption.getComputedTextLength() || caption.textContent.length * 7
			: 0;
		const spot = width > 0 ? placeLabel(points, width, 18, boxes, placedLabels) : undefined;
		caption.setAttribute('x', String(spot?.x ?? 0));
		caption.setAttribute('y', String(spot?.y ?? 0));
		this.routed = route !== undefined;
		this.length = points
			.slice(1)
			.reduce(
				(total, point, index): number =>
					total + Math.hypot(point.x - (points[index]?.x ?? 0), point.y - (points[index]?.y ?? 0)),
				0,
			);
	}
}
