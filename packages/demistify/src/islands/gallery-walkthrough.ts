import { galleryScreenKey } from './gallery-data.ts';
import type { GalleryIsland, GallerySnapshot } from './gallery.ts';

interface GalleryWalkthroughOptions {
	readonly gallery: GalleryIsland;
	readonly player: HTMLElement;
	readonly variationsEnabled: boolean;
	readonly onTarget: (target: HTMLElement) => void;
}

/** The existing route player borrows one gallery and returns its browse state when finished. */
export class GalleryWalkthrough {
	private readonly snapshot: GallerySnapshot;
	private readonly choices = document.createElement('div');
	private readonly observer: ResizeObserver;
	private readonly events = new AbortController();
	private baseId = '';
	private variation: string | undefined;

	readonly gallery: GalleryIsland;
	private readonly player: HTMLElement;
	private readonly variationsEnabled: boolean;
	private readonly onTarget: (target: HTMLElement) => void;
	private readonly restoreHelp: (() => void) | undefined;

	constructor(options: GalleryWalkthroughOptions) {
		const { gallery, player, variationsEnabled, onTarget } = options;
		this.gallery = gallery;
		this.player = player;
		this.variationsEnabled = variationsEnabled;
		this.onTarget = onTarget;
		this.snapshot = gallery.prepareWalkthrough();
		this.variation = new URL(location.href).searchParams.get('ex-variation') ?? undefined;
		gallery.setAttribute('data-walkthrough', '');
		player.setAttribute('data-gallery', '');
		document.documentElement.setAttribute('data-gallery-walkthrough', '');
		const help = gallery.querySelector<HTMLElement>('.ex-gallery-shortcuts');
		const helpHome = help?.parentElement;
		if (help && helpHome) {
			const content = [...help.childNodes];
			document.body.append(help);
			help.innerHTML =
				'<strong>Walkthrough shortcuts</strong><dl><dt>Left / Right</dt><dd>Previous / next step</dd>' +
				(variationsEnabled ? '<dt>Up / Down</dt><dd>Previous / next screen variation</dd>' : '') +
				'<dt>Esc</dt><dd>Close this menu or leave the walkthrough</dd></dl>';
			this.restoreHelp = (): void => {
				helpHome.append(help);
				help.replaceChildren(...content);
			};
		}
		this.choices.className = 'ex-mode-choices ex-gallery-variations';
		this.choices.setAttribute('role', 'group');
		this.choices.setAttribute('aria-label', 'Screen variation');
		if (variationsEnabled) {
			for (const name of this.variationNames()) {
				const button = document.createElement('button');
				button.type = 'button';
				button.textContent = name;
				button.value = name;
				button.addEventListener('click', (): void => this.chooseVariation(name));
				this.choices.append(button);
			}
			if (this.choices.children.length > 1) player.append(this.choices);
		}
		const position = (): void => {
			const bounds = gallery.getBoundingClientRect();
			const left = Math.max(12, bounds.left);
			player.style.left = `${left}px`;
			player.style.width = `${Math.max(0, Math.min(bounds.right, innerWidth - 12) - left)}px`;
			document.documentElement.style.setProperty(
				'--ex-gallery-player-height',
				`${Math.ceil(player.getBoundingClientRect().height)}px`,
			);
			const viewer = gallery.querySelector('.ex-gallery-body');
			if (viewer)
				gallery.style.setProperty(
					'--ex-gallery-viewport-height',
					`${Math.max(160, player.getBoundingClientRect().top - viewer.getBoundingClientRect().top - 16)}px`,
				);
		};
		this.observer = new ResizeObserver(position);
		this.observer.observe(gallery);
		this.observer.observe(player);
		window.addEventListener('resize', position, { signal: this.events.signal });
		window.addEventListener('scroll', position, { signal: this.events.signal, capture: true });
		position();
	}

	private variationNames(): readonly string[] {
		return [
			...new Set(
				this.gallery
					.walkthroughCaptures()
					.map((capture): string => capture.metadata['viewport'] ?? '')
					.filter(Boolean),
			),
		];
	}

	show(baseId: string): HTMLElement | undefined {
		this.baseId = baseId;
		const captures = this.gallery.walkthroughCaptures();
		const base = captures.find((capture): boolean => capture.id === baseId);
		if (!base) return undefined;
		const names = this.variationNames();
		if (this.variation === undefined || !names.includes(this.variation))
			this.variation = base.metadata['viewport'];
		const wanted = this.variationsEnabled ? this.variation : base.metadata['viewport'];
		const matches = this.variationsEnabled
			? captures.filter(
					(capture): boolean =>
						galleryScreenKey(capture) === galleryScreenKey(base) &&
						capture.metadata['viewport'] === wanted,
				)
			: [base];
		for (const button of this.choices.querySelectorAll('button'))
			button.setAttribute('aria-pressed', String(button.value === wanted));
		const address = new URL(location.href);
		if (this.variationsEnabled && wanted) address.searchParams.set('ex-variation', wanted);
		else address.searchParams.delete('ex-variation');
		history.replaceState(null, '', address);
		if (matches.length !== 1) {
			return this.gallery.showUnavailableCapture(
				`${wanted ?? 'Requested'} capture ${matches.length ? 'is ambiguous' : 'is unavailable'}`,
				`${base.metadata['screen'] ?? base.title}. This stays on the same step; choose another variation.`,
			);
		}
		const capture = matches[0];
		return capture ? this.gallery.revealCapture(capture.id) : undefined;
	}

	private chooseVariation(name: string): void {
		this.variation = name;
		const target = this.show(this.baseId);
		if (target) this.onTarget(target);
	}

	moveVariation(direction: number): boolean {
		if (!this.variationsEnabled) return false;
		const names = this.variationNames();
		const next = names[names.findIndex((name): boolean => name === this.variation) + direction];
		if (next !== undefined) this.chooseVariation(next);
		return true;
	}

	dispose(): void {
		this.observer.disconnect();
		this.events.abort();
		this.choices.remove();
		this.restoreHelp?.();
		this.gallery.style.removeProperty('--ex-gallery-viewport-height');
		this.gallery.removeAttribute('data-walkthrough');
		this.player.removeAttribute('data-gallery');
		this.player.style.removeProperty('left');
		this.player.style.removeProperty('width');
		document.documentElement.removeAttribute('data-gallery-walkthrough');
		document.documentElement.style.removeProperty('--ex-gallery-player-height');
		this.gallery.restoreBrowseSnapshot(this.snapshot);
	}
}
