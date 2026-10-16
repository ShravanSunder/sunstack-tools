import { afterEach, expect, test, vi } from 'vitest';

import { createDebouncedAction } from './debounce.ts';

afterEach((): void => {
	vi.useRealTimers();
});

test('a burst publishes only the latest value after 200ms of quiet', (): void => {
	vi.useFakeTimers();
	let query = 'M';
	const published: string[] = [];
	const update = createDebouncedAction((): void => {
		published.push(query);
	}, 200);
	update.schedule();
	vi.advanceTimersByTime(150);
	query = 'Mobile';
	update.schedule();
	vi.advanceTimersByTime(199);
	expect(published).toEqual([]);
	vi.advanceTimersByTime(1);
	expect(published).toEqual(['Mobile']);
	query = 'Laptop';
	update.schedule();
	update.cancel();
	vi.advanceTimersByTime(500);
	expect(published).toEqual(['Mobile']);
});
