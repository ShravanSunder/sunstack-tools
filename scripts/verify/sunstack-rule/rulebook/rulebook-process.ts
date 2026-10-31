import { spawn, spawnSync, type ChildProcessWithoutNullStreams } from 'node:child_process';
/** Target-pinned children and owned event-driven loopback; no polling or real credentials. */
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import http from 'node:http';
import path from 'node:path';

import {
	arrayValue,
	equal,
	errorMessage,
	jsonObjectSchema,
	objectValue,
	parseObject,
	textValue,
	requireCondition,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';

export interface DeclaredTarget {
	binary: string;
	source: string;
	commit: string;
	digest: string;
}
export interface CapturedRequest {
	body: JsonObject;
	raw: Buffer;
}
export interface ControlledReply {
	status: number;
	body: JsonObject;
}
export interface RunCase {
	root: string;
	name: string;
	arguments: string[];
	expectedExit: number;
	credential?: boolean;
	input?: Buffer;
	cancel?: boolean;
}
export interface ProcessResult {
	stdout: Buffer;
	stderr: Buffer;
	captures: CapturedRequest[];
}
export function binaryDigest(binary: string): string {
	filesystem.accessSync(binary, filesystem.constants.X_OK);
	return createHash('sha256').update(filesystem.readFileSync(binary)).digest('hex');
}
export function confirmTarget(target: DeclaredTarget): void {
	requireCondition(/^[0-9a-f]{40}$/.test(target.commit), 'Declare a full Git commit');
	const head = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: target.source, encoding: 'utf8' });
	const status = spawnSync(
		'git',
		[
			'status',
			'--porcelain',
			'--',
			'crates',
			'Cargo.toml',
			'Cargo.lock',
			'rust-toolchain.toml',
			'.cargo',
		],
		{ cwd: target.source, encoding: 'utf8' },
	);
	requireCondition(
		head.status === 0 &&
			status.status === 0 &&
			head.stdout.trim() === target.commit &&
			!status.stdout,
		'Declared product source changed or is dirty',
	);
	equal(binaryDigest(target.binary), target.digest, 'Declared binary bytes changed');
}
export function declaredTarget(options: {
	binary: string;
	source: string;
	commit: string;
	digest?: string;
}): DeclaredTarget {
	const target = {
		binary: path.resolve(options.binary),
		source: path.resolve(options.source),
		commit: options.commit,
		digest: options.digest ?? binaryDigest(options.binary),
	};
	confirmTarget(target);
	return target;
}
function fixtureSnapshot(root: string): Record<string, string> {
	const result: Record<string, string> = {};
	function visit(directory: string): void {
		for (const entry of filesystem.readdirSync(directory, { withFileTypes: true })) {
			if (entry.name === '.git') continue;
			const file = path.join(directory, entry.name);
			if (entry.isDirectory()) visit(file);
			else if (entry.isFile())
				result[path.relative(root, file)] = createHash('sha256')
					.update(filesystem.readFileSync(file))
					.digest('hex');
		}
	}
	visit(root);
	return result;
}
export function responseBody(body: JsonObject, probability = 0.91, choice = 'outcome'): JsonObject {
	const answers: JsonObject = {};
	for (const [name, value] of Object.entries(objectValue(body['questions']))) {
		const question = objectValue(value);
		switch (textValue(question['type'])) {
			case 'noul':
				answers[name] = { type: 'noul', noul: probability };
				break;
			case 'choice': {
				const labels = Object.keys(objectValue(question['criteria']));
				const selected = labels.includes(choice) ? choice : labels[0];
				requireCondition(selected, 'Choice has no labels');
				answers[name] = {
					type: 'choice',
					choice: selected,
					confidence: 1,
					probabilities: Object.fromEntries(
						labels.map((label) => [label, label === selected ? 1 : 0]),
					),
				};
				break;
			}
			case 'score': {
				const criteria = arrayValue(question['criteria']);
				const last = criteria.length - 1;
				requireCondition(last >= 1, 'Score has too few levels');
				answers[name] = {
					type: 'score',
					score: last,
					confidence: 1,
					legend: Object.fromEntries(criteria.map((item, index) => [String(index), item])),
					probabilities: Object.fromEntries(
						criteria.map((_item, index) => [String(index), index === last ? 1 : 0]),
					),
				};
				break;
			}
			default:
				throw new Error('Unrecognized controlled question primitive');
		}
	}
	return {
		model: 'typesafe/controlled-rulebook',
		answers,
		usage: { input_tokens: 20, output_tokens: 5, cost: 0.000001 },
	};
}
export class RulebookProcess {
	readonly target: DeclaredTarget;
	readonly evidence: string;
	readonly captures: CapturedRequest[] = [];
	readonly errors: string[] = [];
	readonly server: http.Server;
	private child: ChildProcessWithoutNullStreams | undefined;
	private cancelOnRequest = false;
	private held = new Set<http.ServerResponse>();
	endpoint = '';
	reply: (body: JsonObject) => JsonObject = responseBody;
	respond: ((request: CapturedRequest) => Promise<ControlledReply>) | undefined;
	readonly events: { request: number; event: 'received' | 'responded' }[] = [];
	constructor(target: DeclaredTarget, evidence: string) {
		this.target = target;
		this.evidence = evidence;
		this.server = http.createServer((request, response) => {
			void this.handle(request, response).catch((error: unknown) => {
				this.errors.push(errorMessage(error));
				if (!response.destroyed) {
					response.writeHead(503);
					response.end('{"error":"controlled response failed"}');
				}
			});
		});
	}
	async start(): Promise<void> {
		confirmTarget(this.target);
		filesystem.mkdirSync(this.evidence, { recursive: false });
		filesystem.mkdirSync(path.join(this.evidence, 'http'));
		await new Promise<void>((resolve, reject) => {
			this.server.once('error', reject);
			this.server.listen(0, '127.0.0.1', () => {
				this.server.off('error', reject);
				resolve();
			});
		});
		const address = this.server.address();
		requireCondition(address && typeof address === 'object', 'Loopback listener has no address');
		this.endpoint = `http://127.0.0.1:${address.port}/systemone`;
	}
	private async handle(
		request: http.IncomingMessage,
		response: http.ServerResponse,
	): Promise<void> {
		const raw = await new Promise<Buffer>((resolve, reject) => {
			const chunks: Buffer[] = [];
			let size = 0;
			request.on('data', (chunk: Buffer) => {
				size += chunk.length;
				if (size > 2_000_000) {
					reject(new Error('Controlled request too large'));
					request.destroy();
				} else chunks.push(chunk);
			});
			request.once('end', () => resolve(Buffer.concat(chunks)));
			request.once('error', reject);
			request.once('aborted', () => reject(new Error('Request body aborted')));
		});
		const body = jsonObjectSchema.parse(JSON.parse(raw.toString('utf8')));
		this.captures.push({ body, raw });
		const number = this.captures.length;
		this.events.push({ request: number, event: 'received' });
		writeJson(path.join(this.evidence, 'http', `http-${number}.json`), body);
		filesystem.writeFileSync(path.join(this.evidence, 'http', `http-${number}.raw.json`), raw);
		if (this.cancelOnRequest) {
			const pid = this.child?.pid;
			requireCondition(pid, 'No owned child for request-triggered cancellation');
			this.held.add(response);
			response.once('close', () => this.held.delete(response));
			process.kill(-pid, 'SIGINT');
			return;
		}
		const reply = this.respond
			? await this.respond({ body, raw })
			: { status: 200, body: this.reply(body) };
		const encoded = Buffer.from(JSON.stringify(reply.body));
		response.writeHead(reply.status, {
			'Content-Type': 'application/json',
			'Content-Length': encoded.length,
		});
		response.end(encoded);
		this.events.push({ request: number, event: 'responded' });
	}
	async run(options: RunCase): Promise<ProcessResult> {
		confirmTarget(this.target);
		const directory = path.join(this.evidence, options.name);
		filesystem.mkdirSync(directory);
		const home = path.join(directory, 'home');
		filesystem.mkdirSync(home);
		const environment: NodeJS.ProcessEnv = {
			PATH: '/usr/bin:/bin',
			HOME: home,
			LANG: 'C.UTF-8',
			GIT_CONFIG_NOSYSTEM: '1',
			GIT_CONFIG_GLOBAL: '/dev/null',
			SUNSTACK_CREDENTIAL_STORE: 'off',
		};
		if (options.credential) environment['OPENROUTER_API_KEY'] = 'SYNTHETIC_RULEBOOK_LOOPBACK_ONLY';
		const before = this.captures.length;
		const snapshot = fixtureSnapshot(options.root);
		this.cancelOnRequest = options.cancel ?? false;
		const command = [this.target.binary, ...options.arguments];
		const child = spawn(this.target.binary, options.arguments, {
			cwd: options.root,
			env: environment,
			detached: true,
			stdio: ['pipe', 'pipe', 'pipe'],
		});
		this.child = child;
		const stdout: Buffer[] = [];
		const stderr: Buffer[] = [];
		child.stdout.on('data', (chunk: Buffer) => stdout.push(chunk));
		child.stderr.on('data', (chunk: Buffer) => stderr.push(chunk));
		let timedOut = false;
		const timer = setTimeout(() => {
			timedOut = true;
			if (child.pid) process.kill(-child.pid, 'SIGKILL');
		}, 30000);
		let ending: { code: number | null; signal: NodeJS.Signals | null };
		try {
			if (options.input) child.stdin.end(options.input);
			ending = await new Promise((resolve, reject) => {
				child.once('error', reject);
				child.once('close', (code, signal) => resolve({ code, signal }));
			});
		} finally {
			clearTimeout(timer);
			child.stdin.destroy();
			this.child = undefined;
		}
		const output = Buffer.concat(stdout);
		const errors = Buffer.concat(stderr);
		const captures = this.captures.slice(before);
		filesystem.writeFileSync(path.join(directory, 'stdout.log'), output);
		filesystem.writeFileSync(path.join(directory, 'stderr.log'), errors);
		writeJson(path.join(directory, 'process.json'), {
			command,
			cwd: options.root,
			pid: child.pid ?? null,
			commit: this.target.commit,
			binary_sha256: this.target.digest,
			exit_code: ending.code,
			signal: ending.signal,
			http_requests: captures.length,
			credential: options.credential ? 'synthetic' : 'absent',
			stdin_held_open: !options.input,
			cancel_on_request: options.cancel ?? false,
			timed_out: timedOut,
		});
		confirmTarget(this.target);
		if (!options.arguments.includes('--init'))
			equal(fixtureSnapshot(options.root), snapshot, 'Lint modified its input fixture');
		requireCondition(!timedOut, `${options.name}: process did not settle in 30 seconds`);
		equal(ending.code, options.expectedExit, `${options.name}: wrong exit; inspect ${directory}`);
		return { stdout: output, stderr: errors, captures };
	}
	async report(options: RunCase): Promise<{ report: JsonObject; captures: CapturedRequest[] }> {
		const result = await this.run(options);
		return { report: parseObject(result.stdout.toString('utf8')), captures: result.captures };
	}
	async close(): Promise<void> {
		for (const response of this.held) response.end('{"error":"owned control closed"}');
		await new Promise<void>((resolve, reject) => {
			const timer = setTimeout(() => reject(new Error('Owned server did not close')), 5000);
			this.server.close((error) => {
				clearTimeout(timer);
				if (error) reject(error);
				else resolve();
			});
			this.server.closeAllConnections();
		});
		writeJson(path.join(this.evidence, 'harness.json'), {
			errors: this.errors,
			requests: this.captures.length,
			events: this.events,
		});
		confirmTarget(this.target);
		requireCondition(
			this.errors.length === 0,
			`Loopback harness errors: ${this.errors.join('; ')}`,
		);
	}
}
