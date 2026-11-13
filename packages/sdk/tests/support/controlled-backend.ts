// A loopback stand-in for the decision-model provider. It replaces only model
// answers: the SDK, addon, Rust encoding, HTTP transport and executor stay real.
import { createServer } from 'node:http';
import type { IncomingMessage, ServerResponse } from 'node:http';
import type { AddressInfo } from 'node:net';

import { ownedDirectory } from './sdk-fixtures.ts';

export interface CapturedRequest {
	readonly body: Readonly<Record<string, unknown>>;
	/** Send the answer for this request. */
	respond(status: number, body: unknown): void;
	/** Whether the client closed the connection before an answer was sent. */
	closedByClient(): boolean;
}

export interface ControlledBackend {
	/** The `decision_model.endpoint` value that routes a configuration here. */
	readonly endpoint: string;
	/** Every request received so far, in arrival order. */
	readonly received: readonly CapturedRequest[];
	/** Resolve with the next request (already received or still to come). */
	nextRequest(): Promise<CapturedRequest>;
	close(): Promise<void>;
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

async function readBody(request: IncomingMessage): Promise<Readonly<Record<string, unknown>>> {
	const chunks: Buffer[] = [];
	for await (const chunk of request)
		chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(String(chunk)));
	const parsed: unknown = JSON.parse(Buffer.concat(chunks).toString('utf8'));
	if (!isRecord(parsed)) throw new Error('decision request body is not an object');
	return parsed;
}

export async function startControlledBackend(): Promise<ControlledBackend> {
	const received: CapturedRequest[] = [];
	let delivered = 0;
	const waiting: ((request: CapturedRequest) => void)[] = [];
	const open = new Set<ServerResponse>();
	const server = createServer((request, response) => {
		open.add(response);
		let answered = false;
		let clientClosed = false;
		response.on('close', () => {
			open.delete(response);
			if (!answered) clientClosed = true;
		});
		void readBody(request).then((body) => {
			const captured: CapturedRequest = {
				body,
				respond(status, answer) {
					answered = true;
					response.writeHead(status, { 'content-type': 'application/json' });
					response.end(JSON.stringify(answer));
				},
				closedByClient: () => clientClosed,
			};
			received.push(captured);
			const waiter = waiting.shift();
			if (waiter !== undefined) {
				delivered += 1;
				waiter(captured);
			}
		});
	});
	await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
	const address: AddressInfo | string | null = server.address();
	if (address === null || typeof address === 'string')
		throw new Error('controlled backend has no port');
	return {
		endpoint: `http://127.0.0.1:${address.port}/v1/systemone`,
		received,
		nextRequest() {
			const ready = received[delivered];
			if (ready !== undefined) {
				delivered += 1;
				return Promise.resolve(ready);
			}
			return new Promise((resolve) => waiting.push(resolve));
		},
		async close() {
			for (const response of open) response.destroy();
			await new Promise<void>((resolve) => server.close(() => resolve()));
		},
	};
}

/** Answer every question in a request with the same no/uncertain/yes probability. */
export function answerNoul(request: CapturedRequest, yesProbability: number): void {
	const questions = request.body['questions'];
	const answers: Record<string, unknown> = {};
	if (isRecord(questions)) {
		for (const name of Object.keys(questions))
			answers[name] = { type: 'noul', noul: yesProbability };
	}
	request.respond(200, {
		model: 'typesafe/controlled',
		answers,
		usage: { input_tokens: 20, output_tokens: 5, cost: 0.001 },
	});
}

/** Answer every question in a request with one chosen label. */
export function answerChoice(
	request: CapturedRequest,
	choice: string,
	labels: readonly string[],
): void {
	const questions = request.body['questions'];
	const answers: Record<string, unknown> = {};
	const probabilities = Object.fromEntries(
		labels.map((label) => [label, label === choice ? 0.9 : 0.1 / (labels.length - 1)]),
	);
	if (isRecord(questions)) {
		for (const name of Object.keys(questions)) {
			answers[name] = { type: 'choice', choice, confidence: 0.9, probabilities };
		}
	}
	request.respond(200, {
		model: 'typesafe/controlled',
		answers,
		usage: { input_tokens: 30, output_tokens: 4 },
	});
}

/** Answers each model request "yes" until stopped. */
export interface Responder {
	/** Stop waiting and settle; resolves with the number answered, or rejects with its failure. */
	stop(): Promise<number>;
}

export function answerEveryRequest(backend: ControlledBackend, responseLimit: number): Responder {
	let release: ((value: 'stopped') => void) | undefined;
	const stopped = new Promise<'stopped'>((resolve) => {
		release = resolve;
	});
	// Every request is answered so no run waits on the responder; a request beyond the
	// limit is recorded as this responder's failure and reported when it stops.
	let overLimit = false;
	const answerFrom = async (answered: number): Promise<number> => {
		const next = await Promise.race([backend.nextRequest(), stopped]);
		if (next === 'stopped') return answered;
		overLimit ||= answered >= responseLimit;
		answerNoul(next, 0.9);
		return answerFrom(answered + 1);
	};
	const settled = answerFrom(0);
	// A failure is reported by `stop`, never left unhandled.
	settled.catch(() => undefined);
	return {
		async stop() {
			release?.('stopped');
			const answered = await settled;
			if (overLimit)
				throw new Error(`${answered} model requests; at most ${responseLimit} expected`);
			return answered;
		},
	};
}

/** What a body run against an answering backend returned, and how many requests it answered. */
export interface AnsweredRun<TValue> {
	readonly value: TValue;
	readonly answered: number;
}

/**
 * Run `body` against a fresh checked repository and a backend that answers at most
 * `responseLimit` requests. The responder is always stopped, the backend always closed and
 * the repository always removed, whatever the body or the responder did; afterwards a body
 * failure, a responder failure, or both, reject.
 */
export async function withAnsweringBackend<TValue>(
	prefix: string,
	responseLimit: number,
	body: (backend: ControlledBackend, root: string) => Promise<TValue>,
): Promise<AnsweredRun<TValue>> {
	const repository = ownedDirectory(prefix);
	const backend = await startControlledBackend();
	const responder = answerEveryRequest(backend, responseLimit);
	process.env['OPENROUTER_API_KEY'] = 'synthetic-controlled-key';
	const ran = await body(backend, repository.path).then(
		(value) => ({ status: 'fulfilled', value }) as const,
		(error: unknown) => ({ status: 'rejected', error }) as const,
	);
	const stopped = await responder.stop().then(
		(answered) => ({ status: 'fulfilled', answered }) as const,
		(error: unknown) => ({ status: 'rejected', error }) as const,
	);
	delete process.env['OPENROUTER_API_KEY'];
	try {
		await backend.close();
	} finally {
		repository.remove();
	}
	if (ran.status === 'rejected' && stopped.status === 'rejected')
		throw new AggregateError([ran.error, stopped.error], 'the body and its responder failed');
	if (ran.status === 'rejected') throw ran.error;
	if (stopped.status === 'rejected') throw stopped.error;
	return { value: ran.value, answered: stopped.answered };
}
