/** Owned answer boundary: real HTTP capture, deliberate statuses and acknowledged holds. */
import http from 'node:http';

import { z } from 'zod';

import { jsonObjectSchema } from '../rulebook/rulebook-fixture-support.ts';
import { responseBody } from '../rulebook/rulebook-process.ts';
import { boundedSdkWait } from '../sdk/sdk-controlled-backend.ts';

export class MissingCredentialBackend {
	readonly captures: string[] = [];
	readonly errors: string[] = [];
	status = 200;
	hold = false;
	private readonly held = new Map<http.ServerResponse, z.infer<typeof jsonObjectSchema>>();
	private readonly waiters = new Set<() => void>();
	private readonly server = http.createServer((request, response) => {
		void this.receive(request, response).catch((error: unknown) => {
			this.errors.push(error instanceof Error ? error.message : 'HTTP capture failed');
			response.destroy();
			for (const wake of this.waiters) wake();
		});
	});
	private endpointValue = '';
	get endpoint(): string {
		if (!this.endpointValue) throw new Error('Backend has not started');
		return this.endpointValue;
	}
	async start(): Promise<void> {
		await new Promise<void>((resolve, reject) => {
			this.server.once('error', reject);
			this.server.listen(0, '127.0.0.1', () => {
				this.server.off('error', reject);
				resolve();
			});
		});
		const address = this.server.address();
		if (!address || typeof address === 'string') throw new Error('No owned loopback address');
		this.endpointValue = `http://127.0.0.1:${address.port}/systemone`;
	}
	private async receive(
		request: http.IncomingMessage,
		response: http.ServerResponse,
	): Promise<void> {
		if (request.method !== 'POST' || request.url !== '/systemone') throw new Error('Wrong route');
		const chunks: Buffer[] = [];
		let size = 0;
		for await (const rawChunk of request) {
			const chunk = z.instanceof(Buffer).parse(rawChunk);
			size += chunk.length;
			if (size > 2_000_000) throw new Error('Controlled request exceeds bound');
			chunks.push(chunk);
		}
		const raw = Buffer.concat(chunks).toString('utf8');
		const body = jsonObjectSchema.parse(JSON.parse(raw));
		this.captures.push(raw);
		if (this.hold) {
			this.held.set(response, body);
			response.once('close', () => this.held.delete(response));
		} else this.answer(response, body);
		for (const wake of this.waiters) wake();
	}
	private answer(response: http.ServerResponse, body: z.infer<typeof jsonObjectSchema>): void {
		response.writeHead(this.status, { 'content-type': 'application/json' });
		response.end(
			JSON.stringify(
				this.status === 200
					? responseBody(body)
					: { error: { message: `Controlled ${this.status}` } },
			),
		);
	}
	async waitForRequests(count: number): Promise<void> {
		let wake: (() => void) | undefined;
		try {
			await boundedSdkWait(
				new Promise<void>((resolve, reject) => {
					wake = () => {
						if (this.errors.length) reject(new Error(this.errors.join('\n')));
						else if (this.captures.length >= count) resolve();
					};
					this.waiters.add(wake);
					wake();
				}),
				`real HTTP witness ${count}`,
			);
		} finally {
			if (wake) this.waiters.delete(wake);
		}
	}
	release(): void {
		this.hold = false;
		for (const [response, body] of this.held) this.answer(response, body);
		this.held.clear();
	}
	async close(): Promise<void> {
		for (const response of this.held.keys()) response.destroy();
		this.held.clear();
		await boundedSdkWait(
			new Promise<void>((resolve, reject) => {
				this.server.close((error) => (error ? reject(error) : resolve()));
				this.server.closeAllConnections();
			}),
			'owned loopback teardown',
		);
	}
}
