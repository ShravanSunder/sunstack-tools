interface InspectionZoom {
	readonly readScale: () => number;
	readonly setScale: (scale: number) => void;
}

/** Modifier gestures belong to the inspection viewport; ordinary scrolling keeps its native job. */
export function installInspectionGestures(viewport: HTMLElement, zoom: InspectionZoom): void {
	viewport.title = 'Cmd+scroll to zoom; Cmd+drag to pan';
	viewport.addEventListener(
		'wheel',
		(event: WheelEvent): void => {
			if (!event.metaKey && !event.ctrlKey) return;
			event.preventDefault();
			const bounds = viewport.getBoundingClientRect();
			const pointerX = event.clientX - bounds.left;
			const pointerY = event.clientY - bounds.top;
			const before = zoom.readScale();
			const anchorX = (viewport.scrollLeft + pointerX) / before;
			const anchorY = (viewport.scrollTop + pointerY) / before;
			const delta =
				event.deltaY *
				(event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? viewport.clientHeight : 1);
			zoom.setScale(before * Math.exp(-delta * 0.002));
			const after = zoom.readScale();
			viewport.scrollLeft = anchorX * after - pointerX;
			viewport.scrollTop = anchorY * after - pointerY;
		},
		{ passive: false },
	);
	let drag:
		| {
				readonly pointerId: number;
				readonly x: number;
				readonly y: number;
				readonly left: number;
				readonly top: number;
		  }
		| undefined;
	viewport.addEventListener('pointerdown', (event: PointerEvent): void => {
		if (event.button !== 0 || (!event.metaKey && !event.ctrlKey)) return;
		event.preventDefault();
		drag = {
			pointerId: event.pointerId,
			x: event.clientX,
			y: event.clientY,
			left: viewport.scrollLeft,
			top: viewport.scrollTop,
		};
		viewport.setPointerCapture(event.pointerId);
		viewport.dataset['panning'] = '';
	});
	viewport.addEventListener('pointermove', (event: PointerEvent): void => {
		if (!drag || event.pointerId !== drag.pointerId) return;
		viewport.scrollLeft = drag.left + drag.x - event.clientX;
		viewport.scrollTop = drag.top + drag.y - event.clientY;
	});
	const stop = (event: PointerEvent): void => {
		if (!drag || event.pointerId !== drag.pointerId) return;
		drag = undefined;
		delete viewport.dataset['panning'];
		if (viewport.hasPointerCapture(event.pointerId))
			viewport.releasePointerCapture(event.pointerId);
	};
	viewport.addEventListener('pointerup', stop);
	viewport.addEventListener('pointercancel', stop);
	viewport.addEventListener('lostpointercapture', stop);
}
