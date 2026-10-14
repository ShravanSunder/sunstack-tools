import { createIconButton } from '../core/icons.ts';
import { reportIssue } from '../core/issues.ts';
import { closeDrawer, isDrawerOpen } from '../islands/drawer.ts';
import { openIsland } from '../islands/inspector.ts';
import { BoardIsland } from './board.ts';
import { revealElement } from './document.ts';

type OpenMode = 'sidebar' | 'modal';

interface RouteStep {
	/** Another page holding the target, or undefined when the target is on this page. */
	readonly page: string | undefined;
	/** The target id on its page; empty for the top of the page. */
	readonly targetId: string;
	readonly narration: string;
	readonly open: OpenMode | undefined;
}

interface Route {
	readonly label: string;
	/** The page that declares the route; tours on other pages read it from there. */
	readonly home: string;
	readonly steps: readonly RouteStep[];
}

/** A page's address without search or hash, for telling pages apart. */
function pageOf(url: URL): string {
	return `${url.protocol}//${url.host}${url.pathname}`;
}

/**
 * Reads the steps of an ex-route element. A step's `to` is an id on the route's home page, or
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
	const children = Array.from(route.querySelectorAll(':scope > ex-step'), (step): RouteStep => {
		const open = step.getAttribute('open');
		return {
			...resolve(step.getAttribute('to') ?? ''),
			narration: step.textContent?.trim() ?? '',
			open: open === 'sidebar' || open === 'modal' ? open : undefined,
		};
	});
	if (children.length > 0) return children;
	return (route.getAttribute('steps') ?? '')
		.split(/\s+/)
		.filter(Boolean)
		.map((to): RouteStep => Object.assign(resolve(to), { narration: '', open: undefined }));
}

/** One step of a route: the island to show, how to open it, and what to say about it. */
export class StepElement extends HTMLElement {}

/** A named path through islands, read from ex-step children or a steps="id id" list. */
export class RouteElement extends HTMLElement {
	readSteps(): readonly RouteStep[] {
		return readRouteSteps(this, new URL(location.href));
	}
}

function findTarget(targetId: string): Element | null {
	return (
		document.getElementById(targetId) ??
		document.querySelector(`ex-mermaid g.node[data-id="${CSS.escape(targetId)}"]`)
	);
}

/**
 * Scrolls a target to the centre and corrects the position while islands above it finish
 * rendering (a diagram that renders during the scroll changes the page height).
 */
function settleIntoView(target: Element, attempt: number = 0): void {
	target.scrollIntoView({ behavior: attempt === 0 ? 'smooth' : 'instant', block: 'center' });
	if (attempt >= 4) return;
	window.setTimeout((): void => {
		const bounds = target.getBoundingClientRect();
		const visible = bounds.top >= 0 && bounds.top < window.innerHeight * 0.6;
		if (!visible) settleIntoView(target, attempt + 1);
	}, 450);
}

/** Floating previous/next player that walks a route, spotlighting and optionally opening each island. */
class RoutePlayer {
	private readonly bar = document.createElement('div');
	private readonly picker = document.createElement('select');
	private readonly counter = document.createElement('span');
	private readonly narration = document.createElement('p');
	private routeIndex = 0;
	private stepIndex = -1;
	private focused: Element | undefined;

	constructor(private readonly routes: readonly Route[]) {
		this.bar.className = 'ex-route-player';
		this.bar.setAttribute('role', 'region');
		this.bar.setAttribute('aria-label', 'Guided route');
		this.bar.hidden = true;
		for (const [index, route] of routes.entries())
			this.picker.add(new Option(route.label, String(index)));
		this.picker.setAttribute('aria-label', 'Route');
		this.picker.hidden = routes.length < 2;
		this.picker.addEventListener('change', (): void => this.start(Number(this.picker.value)));
		const title = document.createElement('strong');
		title.textContent = routes[0]?.label ?? '';
		title.hidden = routes.length > 1;
		this.counter.className = 'ex-route-count';
		this.narration.className = 'ex-route-narration';
		this.narration.setAttribute('aria-live', 'polite');
		const previous = createIconButton('arrow-left', 'Previous step', (): void => this.step(-1));
		const next = createIconButton('arrow-right', 'Next step', (): void => this.step(1));
		const close = createIconButton('x', 'Leave the route', (): void => this.stop());
		const controls = document.createElement('div');
		controls.className = 'ex-route-controls';
		controls.append(this.picker, title, previous, this.counter, next, close);
		this.bar.append(controls, this.narration);
		document.body.append(this.bar);
		document.addEventListener('keydown', (event: KeyboardEvent): void => {
			if (this.bar.hidden || event.target instanceof HTMLSelectElement) return;
			if (event.key === 'ArrowRight') this.step(1);
			else if (event.key === 'ArrowLeft') this.step(-1);
			else if (event.key === 'Escape' && !document.querySelector('dialog[open]') && !isDrawerOpen())
				this.stop();
		});
	}

	start(routeIndex: number, stepIndex = 0): void {
		this.routeIndex = routeIndex;
		this.picker.value = String(routeIndex);
		this.stepIndex = stepIndex - 1;
		this.bar.hidden = false;
		document.documentElement.dataset['route'] = '';
		this.step(1);
	}

	stop(): void {
		this.closeInspector();
		this.focused?.classList.remove('ex-route-focus');
		this.focused = undefined;
		this.bar.hidden = true;
		delete document.documentElement.dataset['route'];
	}

	private closeInspector(): void {
		closeDrawer();
		const dialog = document.querySelector<HTMLDialogElement>('dialog.ex-inspect-dialog[open]');
		if (!dialog) return;
		document.body.append(this.bar);
		dialog.close();
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
			next.searchParams.set('ex-route', route.label);
			next.searchParams.set('ex-step', String(index));
			next.searchParams.set('ex-path', encodeRoute(route));
			next.hash = step.targetId;
			location.assign(next.href);
			return;
		}
		this.stepIndex = index;
		this.closeInspector();
		this.focused?.classList.remove('ex-route-focus');
		const target = step.targetId ? findTarget(step.targetId) : document.body;
		this.counter.textContent = `${index + 1} / ${steps.length}`;
		this.narration.textContent = step.narration;
		this.narration.hidden = step.narration === '';
		if (!target) return;
		revealElement(target);
		target.classList.add('ex-route-focus');
		this.focused = target;
		const board = target.closest('ex-board');
		if (board instanceof BoardIsland && board.isMap()) {
			board.focusOn(target);
			settleIntoView(board);
		} else settleIntoView(target);
		if (step.open && target instanceof HTMLElement) {
			openIsland(target, step.open);
			document.querySelector('dialog.ex-inspect-dialog[open]')?.append(this.bar);
		}
	}
}

/**
 * A tour carries its own route in the address when it moves between pages, so the next page can
 * continue without fetching anything (file:// pages cannot fetch each other).
 */
type EncodedStep = [page: string | null, targetId: string, narration: string, open: OpenMode | 0];

function encodeRoute(route: Route): string {
	const here = location.href.split(/[?#]/)[0] ?? location.href;
	const steps = route.steps.map((step): EncodedStep => [
		step.page ?? here,
		step.targetId,
		step.narration,
		step.open ?? 0,
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
			const [page, targetId, narration, open] = fields;
			const pageUrl = typeof page === 'string' ? page : undefined;
			return {
				page: pageUrl === undefined || pageOf(new URL(pageUrl)) === here ? undefined : pageUrl,
				targetId: typeof targetId === 'string' ? targetId : '',
				narration: typeof narration === 'string' ? narration : '',
				open: open === 'sidebar' ? 'sidebar' : open === 'modal' ? 'modal' : undefined,
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
		const index = localRoutes.findIndex((route): boolean => route.label === label);
		if (player && index >= 0) player.start(index, stepIndex);
		return;
	}
	const carried = decodeRoute(params.get('ex-path') ?? '');
	if (carried) {
		new RoutePlayer([{ label, home, steps: carried }]).start(0, stepIndex);
		return;
	}
	try {
		const response = await fetch(home);
		const source = new DOMParser().parseFromString(await response.text(), 'text/html');
		const element = Array.from(source.querySelectorAll('ex-route')).find(
			(route): boolean => route.getAttribute('label') === label,
		);
		if (!element) throw new Error(`no route "${label}" on ${home}`);
		new RoutePlayer([{ label, home, steps: readRouteSteps(element, new URL(home)) }]).start(
			0,
			stepIndex,
		);
	} catch {
		// file:// pages cannot fetch each other; the reader still gets back to the tour.
		showTourReturn(home, label, stepIndex);
	}
}

/** Adds a route launcher beside the theme toggle when the page defines any routes. */
export function installRoutes(): void {
	const routeElements = Array.from(document.querySelectorAll<RouteElement>('ex-route'));
	const routes = routeElements
		.map((element, index): Route => ({
			label: element.getAttribute('label') ?? `Route ${index + 1}`,
			home: location.href.split(/[?#]/)[0] ?? location.href,
			steps: element.readSteps(),
		}))
		.filter((route): boolean => route.steps.length > 0);
	for (const element of routeElements)
		for (const step of element.readSteps())
			if (step.page === undefined && step.targetId && !findTarget(step.targetId))
				window.setTimeout((): void => {
					if (!findTarget(step.targetId))
						reportIssue(
							element,
							`Step to="${step.targetId}" does not match any island id or diagram node.`,
						);
				}, 2500);
	const player = routes.length > 0 ? new RoutePlayer(routes) : undefined;
	void resumeTour(routes, player);
	if (!player) return;
	const launcher = createIconButton(
		'play',
		`Start guided route: ${routes[0]?.label ?? ''}`,
		(): void => player.start(0),
	);
	launcher.classList.add('ex-route-launcher');
	const label = document.createElement('span');
	label.textContent = routes.length > 1 ? 'Guided routes' : (routes[0]?.label ?? '');
	launcher.append(label);
	document.body.append(launcher);
}
