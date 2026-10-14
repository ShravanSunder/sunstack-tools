export function createPhosphorIcon(iconName: string): HTMLElement {
	const icon = document.createElement('i');
	icon.className = `ph ph-${iconName}`;
	icon.setAttribute('aria-hidden', 'true');
	return icon;
}

export function createIconButton(
	iconName: string,
	label: string,
	action: () => void,
): HTMLButtonElement {
	const button = document.createElement('button');
	button.type = 'button';
	button.className = 'ex-icon-button';
	button.setAttribute('aria-label', label);
	button.title = label;
	button.append(createPhosphorIcon(iconName));
	button.addEventListener('click', action);
	return button;
}
