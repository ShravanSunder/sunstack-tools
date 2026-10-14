import { adoptBareMarkdown } from '../core/bare-markdown.ts';
import { createIconButton } from '../core/icons.ts';
import { createToneIcon, readTone } from '../core/tones.ts';
import { FlowIsland } from './flow.ts';

type BoardMode = 'map' | 'read';
interface Camera {
	scale: number;
	x: number;
	y: number;
}

const regionWidth = 440;
const minimumScale = 0.2;
const maximumScale = 1.6;
const focusScale = 1.1;
const readableScale = 0.6;

/**
 * A titled group of islands on a board. It routes its own arrows like a flow; children marked
 * `detail` stay hidden on the map until the reader opens the region.
 */
export class RegionIsland extends FlowIsland {
	override connectedCallback(): void {
		if (!this.hasAttribute('prepared') && !this.hasAttribute('data-inspector-content')) {
			adoptBareMarkdown(this);
			const tone = readTone(this, undefined);
			const heading = document.createElement('h3');
			heading.className = 'ex-region-head';
			if (tone) heading.append(createToneIcon(tone));
			heading.append(this.getAttribute('title') ?? this.id);
			this.setAttribute(
				'label',
				this.getAttribute('label') ?? this.getAttribute('title') ?? this.id,
			);
			this.removeAttribute('title');
			const details = this.querySelectorAll(':scope > [detail]').length;
			if (details > 0) {
				const more = document.createElement('span');
				more.className = 'ex-region-more';
				more.textContent = `+${details} more`;
				heading.append(more);
			}
			this.prepend(heading);
			this.setAttribute('prepared', '');
		}
		super.connectedCallback();
	}
}

/**
 * A whiteboard of regions: on wide screens a pannable, zoomable map that fits to an overview and
 * drills into a region; on narrow screens, or on request, an ordinary reading flow.
 */
export class BoardIsland extends HTMLElement {
	private canvas: HTMLElement | undefined;
	private camera: Camera = { scale: 1, x: 0, y: 0 };
	private userMoved = false;

	connectedCallback(): void {
		if (this.canvas) return;
		const canvas = document.createElement('ex-flow');
		canvas.className = 'ex-board-canvas';
		const columns = Number.parseInt(this.getAttribute('cols') ?? '3', 10) || 3;
		canvas.setAttribute('cols', String(columns));
		canvas.dataset['designWidth'] = String(columns * regionWidth + (columns - 1) * 72 + 56);
		canvas.append(...Array.from(this.childNodes));
		this.append(this.createToolbar(), canvas);
		this.canvas = canvas;
		this.setMode(this.clientWidth >= 760 && this.getAttribute('mode') !== 'read' ? 'map' : 'read');
		const refit = new ResizeObserver((): void => {
			if (this.dataset['mode'] === 'map' && !this.userMoved) this.fit();
		});
		refit.observe(this);
		refit.observe(canvas);
		this.addEventListener('click', (event: MouseEvent): void => {
			if (this.dataset['mode'] !== 'map' || !(event.target instanceof Element)) return;
			const heading = event.target.closest('.ex-region-head');
			const region = heading?.parentElement;
			if (region) this.focusOn(region);
		});
		this.installPanAndZoom();
	}

	isMap(): boolean {
		return this.dataset['mode'] === 'map';
	}

	/** Opens the element's region and moves the camera to the element. */
	focusOn(target: Element): void {
		if (!this.isMap() || !this.canvas) return;
		const region = target.closest('ex-region');
		for (const open of this.querySelectorAll<HTMLElement>('ex-region[data-open]'))
			if (open !== region) delete open.dataset['open'];
		if (region instanceof HTMLElement) region.dataset['open'] = '';
		requestAnimationFrame((): void => {
			requestAnimationFrame((): void => this.moveCameraTo(target));
		});
	}

	private moveCameraTo(target: Element): void {
		const canvas = this.canvas;
		if (!canvas) return;
		const view = this.getBoundingClientRect();
		const canvasBounds = canvas.getBoundingClientRect();
		const bounds = target.getBoundingClientRect();
		const { scale } = this.camera;
		const left = (bounds.left - canvasBounds.left) / scale;
		const top = (bounds.top - canvasBounds.top) / scale;
		const width = bounds.width / scale;
		const height = bounds.height / scale;
		// Close enough to read, far enough to keep the neighbours in view.
		const nextScale = Math.min(
			focusScale,
			(view.width * 0.86) / width,
			(view.height * 0.86) / height,
		);
		this.userMoved = true;
		this.setCamera({
			scale: nextScale,
			x: view.width / 2 - (left + width / 2) * nextScale,
			y: view.height / 2 - (top + height / 2) * nextScale,
		});
	}

	private createToolbar(): HTMLElement {
		const toolbar = document.createElement('div');
		toolbar.className = 'ex-board-toolbar';
		const modeButton = (mode: BoardMode, icon: string, label: string): HTMLButtonElement => {
			const button = createIconButton(icon, `${label} view`, (): void => this.setMode(mode));
			button.dataset['mode'] = mode;
			button.append(label);
			return button;
		};
		const zoom = (factor: number): void => {
			const view = this.getBoundingClientRect();
			this.zoomAround(view.width / 2, view.height / 2, this.camera.scale * factor);
		};
		const mapOnly = document.createElement('span');
		mapOnly.className = 'ex-board-map-controls';
		mapOnly.append(
			createIconButton('magnifying-glass-minus', 'Zoom out', (): void => zoom(1 / 1.25)),
			createIconButton('magnifying-glass-plus', 'Zoom in', (): void => zoom(1.25)),
			createIconButton('corners-out', 'Fit the whole board', (): void => {
				for (const open of this.querySelectorAll<HTMLElement>('ex-region[data-open]'))
					delete open.dataset['open'];
				this.userMoved = false;
				requestAnimationFrame((): void => this.fit());
			}),
		);
		toolbar.append(
			modeButton('map', 'map-trifold', 'Map'),
			modeButton('read', 'article', 'Read'),
			mapOnly,
		);
		return toolbar;
	}

	private setMode(mode: BoardMode): void {
		const canvas = this.canvas;
		if (!canvas) return;
		this.dataset['mode'] = mode;
		for (const button of this.querySelectorAll<HTMLButtonElement>('.ex-board-toolbar [data-mode]'))
			button.setAttribute('aria-pressed', String(button.dataset['mode'] === mode));
		if (mode === 'map') {
			canvas.style.width = `${canvas.dataset['designWidth'] ?? '1400'}px`;
			this.userMoved = false;
			requestAnimationFrame((): void => this.fit());
		} else {
			canvas.style.removeProperty('width');
			canvas.style.removeProperty('transform');
		}
	}

	private fit(): void {
		const canvas = this.canvas;
		if (!canvas) return;
		const view = this.getBoundingClientRect();
		const width = canvas.offsetWidth;
		const height = canvas.offsetHeight;
		if (width === 0 || height === 0) return;
		// Fit the whole board when it stays readable; otherwise fit the width at a readable
		// floor and start at the top, leaving the rest to pan or drill into.
		const whole = Math.min(1, (view.width * 0.97) / width, (view.height * 0.92) / height);
		const scale =
			whole >= readableScale
				? whole
				: Math.min(1, Math.max(readableScale, (view.width * 0.97) / width));
		this.setCamera({
			scale,
			x: Math.max(12, (view.width - width * scale) / 2),
			y: whole >= readableScale ? Math.max(48, (view.height - height * scale) / 2) : 48,
		});
	}

	private zoomAround(pointX: number, pointY: number, requestedScale: number): void {
		const scale = Math.min(maximumScale, Math.max(minimumScale, requestedScale));
		const { x, y, scale: current } = this.camera;
		this.userMoved = true;
		this.setCamera({
			scale,
			x: pointX - ((pointX - x) / current) * scale,
			y: pointY - ((pointY - y) / current) * scale,
		});
	}

	private setCamera(camera: Camera): void {
		this.camera = camera;
		if (this.canvas)
			this.canvas.style.transform = `translate(${camera.x}px, ${camera.y}px) scale(${camera.scale})`;
	}

	private installPanAndZoom(): void {
		const pointers = new Map<number, { x: number; y: number }>();
		let dragged = false;
		let pinchDistance = 0;
		this.addEventListener('pointerdown', (event: PointerEvent): void => {
			if (!this.isMap() || !(event.target instanceof Element)) return;
			if (event.target.closest('button, a, summary, select, input, .ex-region-head, g.node'))
				return;
			pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
			dragged = false;
		});
		this.addEventListener('pointermove', (event: PointerEvent): void => {
			const previous = pointers.get(event.pointerId);
			if (!previous) return;
			const deltaX = event.clientX - previous.x;
			const deltaY = event.clientY - previous.y;
			if (!dragged && Math.hypot(deltaX, deltaY) < 4) return;
			if (!dragged) {
				dragged = true;
				this.dataset['dragging'] = '';
				this.setPointerCapture(event.pointerId);
			}
			pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
			if (pointers.size === 2) {
				const [first, second] = Array.from(pointers.values());
				if (!first || !second) return;
				const distance = Math.hypot(first.x - second.x, first.y - second.y);
				const view = this.getBoundingClientRect();
				if (pinchDistance > 0)
					this.zoomAround(
						(first.x + second.x) / 2 - view.left,
						(first.y + second.y) / 2 - view.top,
						(this.camera.scale * distance) / pinchDistance,
					);
				pinchDistance = distance;
				return;
			}
			this.userMoved = true;
			this.setCamera({ ...this.camera, x: this.camera.x + deltaX, y: this.camera.y + deltaY });
		});
		const release = (event: PointerEvent): void => {
			pointers.delete(event.pointerId);
			if (pointers.size < 2) pinchDistance = 0;
			if (pointers.size === 0) delete this.dataset['dragging'];
		};
		this.addEventListener('pointerup', release);
		this.addEventListener('pointercancel', release);
		this.addEventListener(
			'click',
			(event: MouseEvent): void => {
				if (!dragged) return;
				dragged = false;
				event.stopPropagation();
				event.preventDefault();
			},
			{ capture: true },
		);
		this.addEventListener(
			'wheel',
			(event: WheelEvent): void => {
				if (!this.isMap() || !(event.ctrlKey || event.metaKey)) return;
				event.preventDefault();
				const view = this.getBoundingClientRect();
				this.zoomAround(
					event.clientX - view.left,
					event.clientY - view.top,
					this.camera.scale * Math.exp(-event.deltaY / 300),
				);
			},
			{ passive: false },
		);
	}
}
