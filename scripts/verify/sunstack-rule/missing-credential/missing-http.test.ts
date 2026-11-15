import { expect, it } from 'vitest';

import { runSequence } from '../rulebook/rulebook-fixture-support.ts';
import { MissingCredentialBackend } from './missing-http.ts';

it('captures complete real request bytes and produces deliberate failure statuses', async () => {
	const backend = new MissingCredentialBackend();
	await backend.start();
	const raw = '{"model":"m","state":{"exact":"café"},"questions":{}}';
	try {
		await runSequence([200, 401, 400, 503], async (status) => {
			backend.status = status;
			const response = await fetch(backend.endpoint, { method: 'POST', body: raw });
			expect(response.status).toBe(status);
			await response.text();
		});
		expect(backend.captures).toEqual([raw, raw, raw, raw]);
		expect(backend.errors).toEqual([]);
	} finally {
		await backend.close();
	}
});

it('acknowledges a held request before release and closes only its own resources', async () => {
	const backend = new MissingCredentialBackend();
	await backend.start();
	backend.hold = true;
	try {
		let completed = false;
		const pending = fetch(backend.endpoint, {
			method: 'POST',
			body: '{"state":{},"questions":{}}',
		});
		void pending.then(() => {
			completed = true;
		});
		await backend.waitForRequests(1);
		expect(completed).toBe(false);
		backend.release();
		const response = await pending;
		expect(response.status).toBe(200);
		await response.text();
	} finally {
		await backend.close();
	}
});

it('settles an aborted held client and tears down without sending a success reply', async () => {
	const backend = new MissingCredentialBackend();
	await backend.start();
	backend.hold = true;
	const controller = new AbortController();
	const pending = fetch(backend.endpoint, {
		method: 'POST',
		body: '{"state":{},"questions":{}}',
		signal: controller.signal,
	});
	const settled = Promise.allSettled([pending]);
	try {
		await backend.waitForRequests(1);
		controller.abort();
		expect((await settled)[0]?.status).toBe('rejected');
		expect(backend.captures).toHaveLength(1);
		expect(backend.errors).toEqual([]);
	} finally {
		controller.abort();
		await settled;
		await backend.close();
	}
});
