import { createIconButton } from '../core/icons.ts';
import { reportIssue } from '../core/issues.ts';
import { closeDrawer, isDrawerOpen } from '../islands/drawer.ts';
import { openIsland } from '../islands/inspector.ts';
import { BoardIsland } from './board.ts';
import { revealElement } from './document.ts';

type OpenMode = 'sidebar' | 'modal';

interface RouteStep {
	readonly targetId: string;
	readonly narration: string;
	readonly open: OpenMode | undefined;
}

interface Route {
	readonly label: string;
	readonly steps: readonly RouteStep[];
}

/** One step of a route: the island to show, how to open it, and what to say about it. */
export class StepElement extends HTMLElement {}

/** A named path through the page's islands, read from ex-step children or a steps="id id" list. */
export class RouteElement extends HTMLElement {
	readSteps(): readonly RouteStep[] {
		const children = Array.from(this.querySelectorAll(':scope > ex-step'), (step): RouteStep => {
			const open = step.getAttribute('open');
			return {
				targetId: step.getAttribute('to') ?? '',
				narration: step.textContent?.trim() ?? '',
				open: open === 'sidebar' || open === 'modal' ? open : undefined,
			};
		});
		if (children.length > 0) return children;
		return (this.getAttribute('steps') ?? '')
			.split(/\s+/)
			.filter(Boolean)
			.map((targetId): RouteStep => ({ targetId, narration: '', open: undefined }));
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

	start(routeIndex: number): void {
		this.routeIndex = routeIndex;
		this.picker.value = String(routeIndex);
		this.stepIndex = -1;
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
		if (!step) return;
		this.stepIndex = index;
		this.closeInspector();
		this.focused?.classList.remove('ex-route-focus');
		const target = findTarget(step.targetId);
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

/** Adds a route launcher beside the theme toggle when the page defines any routes. */
export function installRoutes(): void {
	const routeElements = Array.from(document.querySelectorAll<RouteElement>('ex-route'));
	const routes = routeElements
		.map((element, index): Route => ({
			label: element.getAttribute('label') ?? `Route ${index + 1}`,
			steps: element.readSteps(),
		}))
		.filter((route): boolean => route.steps.length > 0);
	for (const element of routeElements)
		for (const step of element.readSteps())
			if (!findTarget(step.targetId))
				window.setTimeout((): void => {
					if (!findTarget(step.targetId))
						reportIssue(
							element,
							`Step to="${step.targetId}" does not match any island id or diagram node.`,
						);
				}, 2500);
	if (routes.length === 0) return;
	const player = new RoutePlayer(routes);
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
