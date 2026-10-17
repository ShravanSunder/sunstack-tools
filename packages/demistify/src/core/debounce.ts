export interface DebouncedAction {
	readonly schedule: () => void;
	readonly cancel: () => void;
}

/** One trailing update; cancellation also prevents stale work after reset or disconnect. */
export function createDebouncedAction(action: () => void, delayMs: number): DebouncedAction {
	let timer: ReturnType<typeof setTimeout> | undefined;
	const cancel = (): void => {
		if (timer !== undefined) clearTimeout(timer);
		timer = undefined;
	};
	return {
		cancel,
		schedule: (): void => {
			cancel();
			timer = setTimeout((): void => {
				timer = undefined;
				action();
			}, delayMs);
		},
	};
}
