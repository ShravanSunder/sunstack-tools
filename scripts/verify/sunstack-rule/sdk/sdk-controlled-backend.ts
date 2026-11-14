/** Owned HTTP answer stand-in; requests and causal gates stay external to the addon. */
import http from 'node:http';

import { z } from 'zod';

import { jsonObjectSchema } from '../rulebook/rulebook-fixture-support.ts';
import { responseBody } from '../rulebook/rulebook-process.ts';
import { requireSdk } from './sdk-target.ts';

export interface CapturedSdkRequest {
	readonly body: z.infer<typeof jsonObjectSchema>;
	readonly raw: string;
}
export async function boundedSdkWait<TValue>(
	operation: Promise<TValue>,
	label: string,
	waitMilliseconds = 15000,
): Promise<TValue> {
	let timer: ReturnType<typeof setTimeout> | undefined;
	try {
		return await Promise.race([
			operation,
			new Promise<never>((_resolve, reject) => {
				timer = setTimeout(() => reject(new Error(`Timed out: ${label}`)), waitMilliseconds);
			}),
		]);
	} finally {
		clearTimeout(timer);
	}
}
export class SdkControlledBackend {
	readonly captures: CapturedSdkRequest[] = [];
	readonly errors: string[] = [];
	private readonly server: http.Server;
	private readonly held = new Map<http.ServerResponse, z.infer<typeof jsonObjectSchema>>();
	private readonly waiters = new Set<() => void>();
	private endpointValue = '';
	hold = false;
	choiceFor: (body: CapturedSdkRequest['body']) => string = () => 'portable';
	constructor() {
		this.server = http.createServer((request, response) => {
			void this.capture(request, response).catch((error: unknown) => {
				this.errors.push(error instanceof Error ? error.message : 'Controlled HTTP failure');
				response.writeHead(503);
				response.end('{}');
				for (const notify of this.waiters) notify();
			});
		});
	}
	get endpoint(): string {
		requireSdk(this.endpointValue, 'Backend is not listening');
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
		requireSdk(address && typeof address === 'object', 'No loopback address');
		this.endpointValue = `http://127.0.0.1:${address.port}/systemone`;
	}
	private async capture(
		request: http.IncomingMessage,
		response: http.ServerResponse,
	): Promise<void> {
		requireSdk(
			request.method === 'POST' && request.url === '/systemone',
			'Unexpected backend route',
		);
		const chunks: Buffer[] = [];
		let size = 0;
		for await (const chunk of request) {
			const bytes = z.instanceof(Buffer).parse(chunk);
			size += bytes.length;
			requireSdk(size <= 2_000_000, 'Controlled request exceeds bound');
			chunks.push(bytes);
		}
		const raw = Buffer.concat(chunks).toString('utf8');
		const body = jsonObjectSchema.parse(JSON.parse(raw));
		this.captures.push({ body, raw });
		if (this.hold) {
			this.held.set(response, body);
			response.once('close', () => {
				this.held.delete(response);
			});
		} else {
			response.writeHead(200, { 'content-type': 'application/json' });
			response.end(JSON.stringify(responseBody(body, 0.91, this.choiceFor(body))));
		}
		for (const notify of this.waiters) notify();
	}
	async waitForRequests(count: number): Promise<void> {
		let notify: (() => void) | undefined;
		try {
			await boundedSdkWait(
				new Promise<void>((resolve, reject) => {
					notify = () => {
						if (this.errors.length) reject(new Error(this.errors.join('\n')));
						else if (this.captures.length >= count) resolve();
					};
					this.waiters.add(notify);
					notify();
				}),
				`capture ${count} real requests`,
			);
		} finally {
			if (notify) this.waiters.delete(notify);
		}
	}
	async close(): Promise<void> {
		for (const response of this.held.keys()) response.destroy();
		await boundedSdkWait(
			new Promise<void>((resolve, reject) => {
				this.server.close((error) => (error ? reject(error) : resolve()));
				this.server.closeAllConnections();
			}),
			'owned backend close',
		);
	}
	releaseHeld(): void {
		this.hold = false;
		for (const [response, body] of this.held) {
			response.writeHead(200, { 'content-type': 'application/json' });
			response.end(JSON.stringify(responseBody(body, 0.91, this.choiceFor(body))));
		}
		this.held.clear();
	}
}
