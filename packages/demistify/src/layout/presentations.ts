import { createIconButton } from '../core/icons.ts';

interface PresentationMenu {
	readonly button: HTMLButtonElement;
	readonly panel: HTMLElement;
}

/** Named explainers may be local presentations or ordinary relative links to other documents. */
export function createPresentationMenu(navigation: HTMLElement): PresentationMenu | undefined {
	const authored = Array.from(
		navigation.querySelectorAll<HTMLAnchorElement>('[slot="presentations"] a[href]'),
	);
	const presentations = Array.from(document.querySelectorAll<HTMLElement>('ex-presentation'));
	if (presentations.length === 0 && authored.length === 0) return undefined;
	const panel = document.createElement('nav');
	panel.className = 'ex-presentations-panel';
	panel.popover = 'auto';
	panel.setAttribute('aria-label', 'Presentations');
	const title = document.createElement('strong');
	title.textContent = 'Presentations';
	const close = createIconButton('x', 'Close presentations', (): void => panel.hidePopover());
	const header = document.createElement('header');
	header.append(title, close);
	panel.append(header);
	const links: HTMLAnchorElement[] = [];
	for (const presentation of presentations) {
		if (!presentation.id) continue;
		const link = document.createElement('a');
		link.href = `#${presentation.id}`;
		const name = document.createElement('strong');
		name.textContent = presentation.getAttribute('title') ?? presentation.id;
		link.addEventListener('click', (event: MouseEvent): void => {
			event.preventDefault();
			presentation.dispatchEvent(new Event('presentation-start'));
			panel.hidePopover();
		});
		const description = document.createElement('span');
		description.textContent = presentation.getAttribute('description') ?? '';
		link.append(name, description);
		links.push(link);
	}
	for (const link of authored) {
		const copy = link.cloneNode(true);
		if (copy instanceof HTMLAnchorElement) links.push(copy);
	}
	panel.append(...links);
	const button = createIconButton('map-trifold', 'Browse presentations', (): void => {
		if (panel.matches(':popover-open')) panel.hidePopover();
		else panel.showPopover();
	});
	button.classList.add('ex-presentations-trigger');
	panel.addEventListener('click', (event: MouseEvent): void => {
		if (event.target instanceof Element && event.target.closest('a')) panel.hidePopover();
	});
	const updateCurrent = (): void => {
		const selected = document.getElementById(
			document.documentElement.dataset['presentation'] ?? '',
		);
		for (const link of links) {
			if (selected && link.getAttribute('href') === `#${selected.id}`)
				link.setAttribute('aria-current', 'true');
			else link.removeAttribute('aria-current');
		}
	};
	document.addEventListener('presentation-change', updateCurrent);
	updateCurrent();
	document.body.append(panel);
	return { button, panel };
}
