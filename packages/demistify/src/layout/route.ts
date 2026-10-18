import { createIconButton, createPhosphorIcon } from '../core/icons.ts';
import { reportIssue } from '../core/issues.ts';
import { readIslandSource } from '../core/source-text.ts';
import { resolveSubjectIcon } from '../core/subject-icons.ts';
import { closeDrawer, isDrawerOpen } from '../islands/drawer.ts';
import { GalleryWalkthrough } from '../islands/gallery-walkthrough.ts';
import { GalleryIsland } from '../islands/gallery.ts';
import { openIsland } from '../islands/inspector.ts';
import { BoardIsland } from './board.ts';
import { resolveTarget } from './target-resolution.ts';
import { setTrace } from './trace.ts';

type OpenMode = 'sidebar' | 'modal';

interface RouteStep {
	/** Another page holding the target, or undefined when the target is on this page. */
	readonly page: string | undefined;
	/** The target id on its page; empty for the top of the page. */
	readonly targetId: string;
	readonly narration: string;
	readonly open: OpenMode | undefined;
	readonly icon: string | undefined;
}

interface Route {
	readonly id: string;
	readonly description: string;
	readonly label: string;
	/** The page that declares the route; tours on other pages read it from there. */
	readonly home: string;
	readonly steps: readonly RouteStep[];
	readonly variations: boolean;
}

/** A page's address without search or hash, for telling pages apart. */
function pageOf(url: URL): string {
	return `${url.protocol}//${url.host}${url.pathname}`;
}

/**
 * Reads the steps of an ex-presentation element. A step's `to` is an id on the route's home page, or
 * another page (`registry.html#resume`), which makes the route a tour across pages.
 */
function readRouteSteps(route: Element, home: URL): RouteStep[] {
	const here = pageOf(new URL(location.href));
	const resolve = (to: string): Pick<RouteStep, 'page' | 'targetId'> => {
		const url = /\.html?($|[?#])/i.test(to) ? new URL(to, home) : new URL(`#${to}`, home);
		const page = pageOf(url);
		return {
			page: page === here ? undefined : page,
			targetId: decodeURIComponent(url.hash.slice(1)),
		};
	};
	const children = Array.from(
		route.querySelectorAll<HTMLElement>(':scope > ex-step'),
		(step): RouteStep => {
			const open = step.getAttribute('open');
			return {
				...resolve(step.getAttribute('to') ?? ''),
				narration: readIslandSource(step)?.trim() ?? step.textContent?.trim() ?? '',
				open: open === 'sidebar' || open === 'modal' ? open : undefined,
				icon: resolveSubjectIcon(step.getAttribute('icon') ?? ''),
			};
		},
	);
	return children;
}

/** One step of a route: the island to show, how to open it, and what to say about it. */
export class StepElement extends HTMLElement {}

/** A named path through islands, read from ex-step children or a steps="id id" list. */
export class RouteElement extends HTMLElement {
	connectedCallback(): void {
		this.hidden = true;
	}
	readSteps(): readonly RouteStep[] {
		return readRouteSteps(this, new URL(location.href));
	}
}

function settleIntoView(target: Element): void {
	target.scrollIntoView({ behavior: 'instant', block: 'center' });
}

/** Floating previous/next player that walks a route, spotlighting and optionally opening each island. */
class RoutePlayer {
	private readonly bar = document.createElement('div');
	private readonly title = document.createElement('strong');
	private readonly counter = document.createElement('span');
	private readonly narration = document.createElement('div');
	private readonly marker = document.createElement('span');
	private pendingTarget: AbortController | undefined;
	private routeIndex = 0;
	private stepIndex = -1;
	private focused: Element | undefined;
	private tracedDiagram: Element | undefined;
	private galleryWalkthrough: GalleryWalkthrough | undefined;

	constructor(private readonly routes: readonly Route[]) {
		this.bar.className = 'ex-route-player';
		this.bar.setAttribute('role', 'region');
		this.bar.setAttribute('aria-label', 'Walkthrough player');
		this.marker.className = 'ex-presentation-marker';
		this.marker.setAttribute('aria-hidden', 'true');
		this.marker.hidden = true;
		const positionMarker = (): void => {
			if (!this.focused || this.bar.hidden) return;
			const bounds = this.focused.getBoundingClientRect();
			const gutter = this.focused.closest('ex-mermaid')?.getBoundingClientRect() ?? bounds;
			this.marker.style.left = `${Math.max(6, gutter.left - 15)}px`;
			this.marker.style.top = `${bounds.top + Math.min(16, bounds.height / 2)}px`;
			this.marker.hidden = bounds.bottom < 0 || bounds.top > innerHeight;
		};
		window.addEventListener('scroll', positionMarker, { passive: true, capture: true });
		window.addEventListener('resize', positionMarker);
		document.addEventListener('presentation-target', positionMarker);
		this.bar.hidden = true;
		this.title.textContent = routes[0]?.label ?? '';
		this.counter.className = 'ex-route-count';
		this.narration.className = 'ex-route-narration';
		this.narration.setAttribute('aria-live', 'polite');
		const previous = createIconButton('arrow-left', 'Previous step', (): void => this.step(-1));
		const next = createIconButton('arrow-right', 'Next step', (): void => this.step(1));
		const close = createIconButton('x', 'Close walkthrough', (): void => this.stop());
		const controls = document.createElement('div');
		controls.className = 'ex-route-controls';
		controls.append(this.title, previous, this.counter, next, close);
		this.bar.append(controls, this.narration);
		document.body.append(this.bar, this.marker);
		new ResizeObserver((): void => this.measurePlayerHeight()).observe(this.bar);
		document.addEventListener('keydown', (event: KeyboardEvent): void => {
			if (
				this.bar.hidden ||
				event.defaultPrevented ||
				event.ctrlKey ||
				event.metaKey ||
				event.altKey ||
				(event.target instanceof Element &&
					event.target.closest(
						'input,select,textarea,[contenteditable="true"],[popover],[role="tab"],[role="menuitem"],[role="separator"],.ex-drawer,dialog',
					))
			)
				return;
			if (document.querySelector(':popover-open')) return;
			if (event.key === 'ArrowRight' || event.key === 'ArrowLeft') {
				event.preventDefault();
				this.step(event.key === 'ArrowRight' ? 1 : -1);
			} else if (
				this.galleryWalkthrough &&
				(event.key === 'ArrowUp' || event.key === 'ArrowDown')
			) {
				if (this.galleryWalkthrough.moveVariation(event.key === 'ArrowUp' ? -1 : 1))
					event.preventDefault();
			} else if (
				event.key === 'Escape' &&
				!document.querySelector('dialog[open]') &&
				!isDrawerOpen()
			)
				this.stop();
		});
	}

	start(routeIndex: number, stepIndex = 0): void {
		this.galleryWalkthrough?.dispose();
		this.galleryWalkthrough = undefined;
		this.routeIndex = routeIndex;
		this.title.textContent = this.routes[routeIndex]?.label ?? '';
		this.stepIndex = stepIndex - 1;
		this.bar.hidden = false;
		document.documentElement.dataset['route'] = '';
		this.step(1);
		document.documentElement.dataset['presentation'] = this.routes[routeIndex]?.id ?? '';
		document.dispatchEvent(new Event('presentation-change'));
	}

	stop(): void {
		this.pendingTarget?.abort();
		this.galleryWalkthrough?.dispose();
		this.galleryWalkthrough = undefined;
		this.marker.hidden = true;
		delete document.documentElement.dataset['presentation'];
		document.dispatchEvent(new Event('presentation-change'));
		if (this.tracedDiagram) setTrace(this.tracedDiagram, undefined);
		this.tracedDiagram = undefined;
		this.closeInspector();
		this.focused?.classList.remove('ex-route-focus');
		this.focused = undefined;
		this.bar.hidden = true;
		delete document.documentElement.dataset['route'];
		const address = new URL(location.href);
		for (const key of [
			'ex-tour',
			'ex-route',
			'ex-name',
			'ex-step',
			'ex-path',
			'ex-variation',
			'ex-variations',
		])
			address.searchParams.delete(key);
		history.replaceState(null, '', address);
	}

	private closeInspector(): void {
		closeDrawer();
		const dialog = document.querySelector<HTMLDialogElement>('dialog.ex-inspect-dialog[open]');
		if (!dialog) return;
		document.body.append(this.bar);
		dialog.close();
	}

	private measurePlayerHeight(): void {
		document.documentElement.style.setProperty(
			'--ex-route-player-height',
			`${Math.ceil(this.bar.getBoundingClientRect().height)}px`,
		);
	}

	private async renderNarration(step: RouteStep, home: string, signal: AbortSignal): Promise<void> {
		this.narration.textContent = step.narration;
		this.narration.hidden = step.narration === '';
		if (!step.narration) return;
		const { renderNarrationContent } = await import('../render/markdown-content.ts');
		if (signal.aborted) return;
		this.narration.innerHTML = renderNarrationContent(step.narration, home);
		if (step.icon) {
			const lead = this.narration.querySelector('p');
			if (lead) {
				const content = document.createElement('span');
				content.append(...lead.childNodes);
				lead.classList.add('ex-route-lead');
				lead.replaceChildren(createPhosphorIcon(step.icon), content);
			}
		}
	}

	private step(direction: number): void {
		const steps = this.routes[this.routeIndex]?.steps ?? [];
		const index = Math.min(steps.length - 1, Math.max(0, this.stepIndex + direction));
		const step = steps[index];
		const route = this.routes[this.routeIndex];
		if (!step || !route) return;
		if (step.page !== undefined && index !== this.stepIndex) {
			// The step lives on another page: go there and carry the tour along in the address.
			const next = new URL(step.page);
			next.searchParams.set('ex-tour', route.home);
			next.searchParams.set('ex-route', route.id);
			next.searchParams.set('ex-name', route.label);
			next.searchParams.set('ex-step', String(index));
			next.searchParams.set('ex-path', encodeRoute(route));
			if (route.variations) {
				next.searchParams.set('ex-variations', 'true');
				const variation = new URL(location.href).searchParams.get('ex-variation');
				if (variation) next.searchParams.set('ex-variation', variation);
			}
			next.hash = step.targetId;
			location.assign(next.href);
			return;
		}
		this.stepIndex = index;
		this.closeInspector();
		this.focused?.classList.remove('ex-route-focus');
		this.focused = undefined;
		this.marker.hidden = true;
		this.pendingTarget?.abort();
		const pending = new AbortController();
		this.pendingTarget = pending;
		this.counter.textContent = `${index + 1} / ${steps.length}`;
		void this.renderNarration(step, route.home, pending.signal).catch((error: unknown): void => {
			if (!pending.signal.aborted)
				reportIssue(this.bar, `Walkthrough narration could not render: ${String(error)}`);
		});
		const address = new URL(location.href);
		address.searchParams.set('ex-tour', route.home);
		address.searchParams.set('ex-route', route.id);
		address.searchParams.set('ex-name', route.label);
		address.searchParams.set('ex-step', String(index));
		address.hash = step.targetId;
		if (address.href !== location.href) history.pushState(null, '', address);
		const gallery = [...document.querySelectorAll('ex-gallery')].find(
			(owner): owner is GalleryIsland =>
				owner instanceof GalleryIsland &&
				owner.walkthroughCaptures().some((capture): boolean => capture.id === step.targetId),
		);
		if (this.galleryWalkthrough?.gallery !== gallery) {
			this.galleryWalkthrough?.dispose();
			this.galleryWalkthrough = gallery
				? new GalleryWalkthrough({
						gallery,
						player: this.bar,
						variationsEnabled: route.variations,
						onTarget: (target): void => {
							this.focused?.classList.remove('ex-route-focus');
							this.focused = target;
							target.classList.add('ex-route-focus');
							document.dispatchEvent(new Event('presentation-target'));
						},
					})
				: undefined;
		}
		const galleryTarget = this.galleryWalkthrough?.show(step.targetId);
		void (
			galleryTarget
				? Promise.resolve(galleryTarget)
				: step.targetId
					? resolveTarget(step.targetId, pending.signal)
					: Promise.resolve(document.body)
		).then((target): void => {
			if (pending.signal.aborted) return;
			if (!target) {
				pending.abort();
				this.narration.hidden = false;
				this.narration.textContent = `Target unavailable: ${step.targetId}`;
				return;
			}
			target.classList.add('ex-route-focus');
			this.focused = target;
			// Reserve the actual gutter before bottom-clamped positioning, not on the next resize tick.
			this.measurePlayerHeight();
			// A step on a diagram node lights the node and its connections: a walk through the diagram.
			if (this.tracedDiagram) setTrace(this.tracedDiagram, undefined);
			this.tracedDiagram = undefined;
			const diagram = target.matches('g[data-id]') ? target.closest('svg') : null;
			if (diagram && target instanceof SVGElement) {
				setTrace(diagram, target.dataset['id']);
				this.tracedDiagram = diagram;
			}
			const board = target.closest('ex-board');
			if (board instanceof BoardIsland && board.isMap()) {
				board.focusOn(target);
				settleIntoView(board);
			} else if (gallery) gallery.scrollIntoView({ behavior: 'instant', block: 'start' });
			else settleIntoView(target);
			if (step.open && !gallery && target instanceof HTMLElement) {
				openIsland(target, step.open);
				document.querySelector('dialog.ex-inspect-dialog[open]')?.append(this.bar);
			}
			document.dispatchEvent(new Event('presentation-target'));
		});
	}
}

/**
 * A tour carries its own route in the address when it moves between pages, so the next page can
 * continue without fetching anything (file:// pages cannot fetch each other).
 */
type EncodedStep = [
	page: string | null,
	targetId: string,
	narration: string,
	open: OpenMode | 0,
	icon: string | null,
];

function encodeRoute(route: Route): string {
	const here = location.href.split(/[?#]/)[0] ?? location.href;
	const steps = route.steps.map((step): EncodedStep => [
		step.page ?? here,
		step.targetId,
		step.narration,
		step.open ?? 0,
		step.icon ?? null,
	]);
	const bytes = new TextEncoder().encode(JSON.stringify(steps));
	return btoa(String.fromCodePoint(...bytes))
		.replaceAll('+', '-')
		.replaceAll('/', '_')
		.replace(/=+$/, '');
}

function decodeRoute(encoded: string): RouteStep[] | undefined {
	try {
		const binary = atob(encoded.replaceAll('-', '+').replaceAll('_', '/'));
		const text = new TextDecoder().decode(
			Uint8Array.from(binary, (character): number => character.codePointAt(0) ?? 0),
		);
		const steps: unknown = JSON.parse(text);
		if (!Array.isArray(steps)) return undefined;
		const here = pageOf(new URL(location.href));
		return steps.map((entry: unknown): RouteStep => {
			const fields: readonly unknown[] = Array.isArray(entry) ? (entry as readonly unknown[]) : [];
			const [page, targetId, narration, open, icon] = fields;
			const pageUrl = typeof page === 'string' ? page : undefined;
			return {
				page: pageUrl === undefined || pageOf(new URL(pageUrl)) === here ? undefined : pageUrl,
				targetId: typeof targetId === 'string' ? targetId : '',
				narration: typeof narration === 'string' ? narration : '',
				open: open === 'sidebar' ? 'sidebar' : open === 'modal' ? 'modal' : undefined,
				icon: typeof icon === 'string' ? resolveSubjectIcon(icon) : undefined,
			};
		});
	} catch {
		return undefined;
	}
}

/** A tour that arrived from another page but could not read its route: offer the way back. */
function showTourReturn(home: string, label: string, stepIndex: number): void {
	const back = new URL(home);
	back.searchParams.set('ex-tour', home);
	back.searchParams.set('ex-route', label);
	back.searchParams.set('ex-step', String(stepIndex + 1));
	const bar = document.createElement('div');
	bar.className = 'ex-route-player';
	const link = document.createElement('a');
	link.href = back.href;
	link.textContent = `Continue "${label}" →`;
	bar.append(link);
	document.body.append(bar);
}

/** Continues a tour that this page was opened from, reading the route from its home page. */
async function resumeTour(
	localRoutes: readonly Route[],
	player: RoutePlayer | undefined,
): Promise<void> {
	const params = new URLSearchParams(location.search);
	const home = params.get('ex-tour');
	const label = params.get('ex-route');
	const stepIndex = Number.parseInt(params.get('ex-step') ?? '0', 10) || 0;
	if (!home || label === null) return;
	if (pageOf(new URL(home)) === pageOf(new URL(location.href))) {
		const index = localRoutes.findIndex((route): boolean => route.id === label);
		if (player && index >= 0) player.start(index, stepIndex);
		return;
	}
	const carried = decodeRoute(params.get('ex-path') ?? '');
	if (carried) {
		new RoutePlayer([
			{
				id: label,
				label: params.get('ex-name') ?? label,
				description: '',
				home,
				steps: carried,
				variations: params.get('ex-variations') === 'true',
			},
		]).start(0, stepIndex);
		return;
	}
	try {
		const response = await fetch(home);
		const source = new DOMParser().parseFromString(await response.text(), 'text/html');
		const element = Array.from(source.querySelectorAll('ex-presentation')).find(
			(route): boolean => route.id === label,
		);
		if (!element) throw new Error(`no route "${label}" on ${home}`);
		new RoutePlayer([
			{
				id: label,
				label: element.getAttribute('title') ?? label,
				description: element.getAttribute('description') ?? '',
				home,
				steps: readRouteSteps(element, new URL(home)),
				variations: element.hasAttribute('variations'),
			},
		]).start(0, stepIndex);
	} catch {
		// file:// pages cannot fetch each other; the reader still gets back to the tour.
		showTourReturn(home, label, stepIndex);
	}
}

/** Presentation paths share one launcher/menu with document navigation. */
export function installRoutes(): void {
	const elements = Array.from(document.querySelectorAll<RouteElement>('ex-presentation'));
	const routes = elements.map((element, index): Route => ({
		id: element.id || `presentation-${index + 1}`,
		label: element.getAttribute('title') ?? element.id,
		description: element.getAttribute('description') ?? '',
		home: location.href.split(/[?#]/)[0] ?? location.href,
		steps: element.readSteps(),
		variations: element.hasAttribute('variations'),
	}));
	const player = routes.length ? new RoutePlayer(routes) : undefined;
	for (const [index, element] of elements.entries())
		element.addEventListener('presentation-start', (): void => player?.start(index));
	const initial = elements.findIndex((element): boolean => `#${element.id}` === location.hash);
	if (initial >= 0) player?.start(initial);
	else void resumeTour(routes, player);
	window.addEventListener('popstate', (): void => {
		if (new URLSearchParams(location.search).has('ex-route')) void resumeTour(routes, player);
		else player?.stop();
	});
}
