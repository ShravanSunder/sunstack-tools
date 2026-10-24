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

/** Phosphor Icons 2.1.2 chat-text regular/fill, MIT; license ships with the shared assets. */
export function createCommentIcon(filled = false): SVGSVGElement {
	const icon = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
	icon.setAttribute('viewBox', '0 0 1024 1024');
	icon.setAttribute('aria-hidden', 'true');
	icon.classList.add('ex-comment-symbol');
	icon.classList.toggle('ex-comment-symbol-filled', filled);
	const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
	path.setAttribute(
		'd',
		filled
			? 'M864 192h-704c-35.346 0-64 28.654-64 64v0 640c-0.001 0.114-0.001 0.248-0.001 0.383 0 25.435 14.987 47.369 36.611 57.454l0.39 0.163c7.914 3.774 17.197 5.984 26.995 6l0.005 0c15.684-0.039 30.025-5.752 41.088-15.194l-0.088 0.074 0.36-0.28 130.64-112.6h532c35.346 0 64-28.654 64-64v0-512c0-35.346-28.654-64-64-64v0zM640 608h-256c-17.673 0-32-14.327-32-32s14.327-32 32-32v0h256c17.673 0 32 14.327 32 32s-14.327 32-32 32v0zM640 480h-256c-17.673 0-32-14.327-32-32s14.327-32 32-32v0h256c17.673 0 32 14.327 32 32s-14.327 32-32 32v0z'
			: 'M864 192h-704c-35.346 0-64 28.654-64 64v0 640c-0.001 0.112-0.001 0.244-0.001 0.376 0 25.426 14.967 47.356 36.572 57.461l0.389 0.163c7.927 3.77 17.223 5.979 27.033 6l0.007 0c15.684-0.039 30.025-5.752 41.088-15.194l-0.088 0.074 0.36-0.28 130.64-112.6h532c35.346 0 64-28.654 64-64v0-512c0-35.346-28.654-64-64-64v0zM160 896v0zM864 768h-544c-8.019 0.004-15.347 2.957-20.959 7.833l0.039-0.033-139.080 120.2v-640h704zM352 448c0-17.673 14.327-32 32-32v0h256c17.673 0 32 14.327 32 32s-14.327 32-32 32v0h-256c-17.673 0-32-14.327-32-32v0zM352 576c0-17.673 14.327-32 32-32v0h256c17.673 0 32 14.327 32 32s-14.327 32-32 32v0h-256c-17.673 0-32-14.327-32-32v0z',
	);
	icon.append(path);
	return icon;
}
