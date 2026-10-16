/** Real local workerd/Assets/R2 trial. This command has no remote-deploy or login path. */
import { spawn, execFile } from 'node:child_process';
import { createWriteStream } from 'node:fs';
import { chmod, cp, mkdir, mkdtemp, readFile, readdir, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const executeFile = promisify(execFile);
const repositoryRoot = fileURLToPath(new URL('../../../', import.meta.url));
const [wranglerArgument, portArgument] = process.argv
	.slice(2)
	.filter((argument): boolean => argument !== '--serve');
if (!wranglerArgument)
	throw new Error(
		'Usage: node apps/demistify-poc/scripts/probe-cloudflare-local.ts WRANGLER_JS [BASE_PORT] [--serve]',
	);
const wranglerEntry = resolve(wranglerArgument);
await readFile(wranglerEntry);
const basePort = Number(portArgument ?? 4193);
if (!Number.isInteger(basePort) || basePort < 1024 || basePort > 65534)
	throw new Error('BASE_PORT must be 1024–65534.');
const trialRoot = await mkdtemp(join(tmpdir(), '2026-10-demistify-delivery-'));
await chmod(trialRoot, 0o700);
const libraryRoot = join(repositoryRoot, 'packages/demistify/dist');
const releaseNames = ['0.1.0', '0.1.1'];
const environment = { ...process.env, WRANGLER_SEND_METRICS: 'false' };
const children: ReturnType<typeof spawn>[] = [];
const mimeTypes: Readonly<Record<string, string>> = {
	'.js': 'text/javascript',
	'.css': 'text/css',
	'.json': 'application/json',
	'.woff2': 'font/woff2',
};
const sharedWorker = `export default { async fetch(request, env) {
 const url = new URL(request.url);
 const headers = new Headers({'Access-Control-Allow-Origin':'*','Timing-Allow-Origin':'*'});
 if(url.pathname.startsWith('/0.1/')) { headers.set('Location','/0.1.0/'+url.pathname.slice(5)); headers.set('Cache-Control','no-store'); return new Response(null,{status:302,headers}); }
 BACKEND
}};`;
const assetsWorker = sharedWorker.replace(
	'BACKEND',
	`const response = await env.ASSETS.fetch(request);
 for(const [name,value] of response.headers)headers.set(name,value);
 headers.set('Cache-Control',response.status===200||response.status===304?'public, max-age=31536000, immutable':'no-store');
 return new Response(response.body,{status:response.status,headers});`,
);
const r2Worker = sharedWorker.replace(
	'BACKEND',
	`const object=await env.RELEASES.get(url.pathname.slice(1));
 if(!object){headers.set('Cache-Control','no-store');return new Response('Not found',{status:404,headers});}
 object.writeHttpMetadata(headers);headers.set('ETag',object.httpEtag);headers.set('Cache-Control','public, max-age=31536000, immutable');
 if(request.headers.get('If-None-Match')===object.httpEtag)return new Response(null,{status:304,headers});
 return new Response(request.method==='HEAD'?null:object.body,{headers});`,
);

async function prepareVariant(variant: 'assets' | 'r2'): Promise<string> {
	const directory = join(trialRoot, variant);
	await mkdir(directory);
	await writeFile(join(directory, 'worker.mjs'), variant === 'assets' ? assetsWorker : r2Worker);
	const common = {
		name: `demistify-${variant}-local-probe`,
		main: 'worker.mjs',
		compatibility_date: '2026-10-15',
		workers_dev: false,
		send_metrics: false,
	};
	const binding =
		variant === 'assets'
			? {
					assets: {
						directory: 'public',
						binding: 'ASSETS',
						run_worker_first: true,
						not_found_handling: 'none',
					},
				}
			: {
					r2_buckets: [
						{ binding: 'RELEASES', bucket_name: 'demistify-local-releases', remote: false },
					],
				};
	await writeFile(
		join(directory, 'wrangler.json'),
		JSON.stringify({ ...common, ...binding }, null, 2),
	);
	return directory;
}
const assetsRoot = await prepareVariant('assets');
const r2Root = await prepareVariant('r2');
for (const release of releaseNames) {
	// oxlint-disable-next-line no-await-in-loop -- publish complete roots before starting either reader.
	await cp(libraryRoot, join(assetsRoot, 'public', release), {
		recursive: true,
		errorOnExist: true,
		force: false,
	});
}
const names = (await readdir(libraryRoot, { recursive: true })).filter((name): boolean =>
	Boolean(extname(name)),
);

async function seedLocalObject(release: string, name: string): Promise<void> {
	const key = `${release}/${name}`;
	const result = await executeFile(
		process.execPath,
		[
			wranglerEntry,
			'r2',
			'object',
			'put',
			`demistify-local-releases/${key}`,
			'--file',
			join(libraryRoot, name),
			'--local',
			'--config',
			join(r2Root, 'wrangler.json'),
			'--persist-to',
			join(r2Root, 'state'),
			'--content-type',
			mimeTypes[extname(name)] ?? 'application/octet-stream',
		],
		{ cwd: trialRoot, env: environment, maxBuffer: 2_000_000 },
	);
	await writeFile(join(trialRoot, 'seed-output.txt'), result.stdout + result.stderr, {
		flag: 'a',
		mode: 0o600,
	});
}
for (const release of releaseNames)
	for (const name of names) {
		// oxlint-disable-next-line no-await-in-loop -- one CLI writer seeds the shared local R2 state.
		await seedLocalObject(release, name);
	}

function startLocalVariant(directory: string, port: number): Promise<void> {
	return new Promise((resolveReady, reject): void => {
		const output = createWriteStream(join(directory, 'runtime-output.txt'), { mode: 0o600 });
		const child = spawn(
			process.execPath,
			[
				wranglerEntry,
				'dev',
				'--local',
				'--config',
				join(directory, 'wrangler.json'),
				'--inspector-port',
				'0',
				'--ip',
				'127.0.0.1',
				'--port',
				String(port),
				'--persist-to',
				join(directory, 'state'),
			],
			{ cwd: trialRoot, env: environment, stdio: ['ignore', 'pipe', 'pipe'] },
		);
		children.push(child);
		let tail = '';
		let ready = false;
		const deadline = setTimeout(
			(): void =>
				reject(
					new Error(
						'Local Wrangler did not report readiness within 30 seconds; inspect its private output.',
					),
				),
			30_000,
		);
		child.stdout?.on('data', (chunk: Buffer): void => {
			output.write(chunk);
			tail = (tail + chunk.toString('utf8')).slice(-16_000);
			if (tail.includes(`Ready on http://127.0.0.1:${port}`)) {
				ready = true;
				clearTimeout(deadline);
				resolveReady();
			}
		});
		child.stderr?.on('data', (chunk: Buffer): void => {
			output.write(chunk);
		});
		child.once('error', (error: Error): void => {
			clearTimeout(deadline);
			reject(error);
		});
		child.once('exit', (code: number | null): void => {
			clearTimeout(deadline);
			output.end();
			if (!ready) reject(new Error(`Local Wrangler exited ${code}; inspect its private output.`));
		});
	});
}
interface FileObservation {
	readonly name: string;
	readonly status: number;
	readonly exactBytes: boolean;
	readonly cors: boolean;
	readonly immutable: boolean;
	readonly mime: string | null;
}
async function probeVariant(port: number): Promise<Record<string, unknown>> {
	const origin = `http://127.0.0.1:${port}`;
	const files = await Promise.all(
		releaseNames.flatMap((release) =>
			names.map(async (name): Promise<FileObservation> => {
				const response = await fetch(`${origin}/${release}/${name}`);
				return {
					name: `${release}/${name}`,
					status: response.status,
					exactBytes: Buffer.from(await response.arrayBuffer()).equals(
						await readFile(join(libraryRoot, name)),
					),
					cors: response.headers.get('Access-Control-Allow-Origin') === '*',
					immutable: response.headers.get('Cache-Control')?.includes('immutable') === true,
					mime: response.headers.get('Content-Type'),
				};
			}),
		),
	);
	const initial = await fetch(`${origin}/0.1.0/runtime.js`);
	await initial.arrayBuffer();
	const etag = initial.headers.get('ETag');
	const conditional = await fetch(`${origin}/0.1.0/runtime.js`, {
		headers: { 'If-None-Match': etag ?? '' },
	});
	const missing = await fetch(`${origin}/0.1.0/missing.js`);
	const redirect = await fetch(`${origin}/0.1/runtime.js`, { redirect: 'manual' });
	const checks = {
		completePinnedRoots: files.every(
			(file): boolean => file.status === 200 && file.exactBytes && file.cors && file.immutable,
		),
		knownMimeTypes: files.every(
			(file): boolean =>
				file.mime?.startsWith(mimeTypes[extname(file.name)] ?? 'application/octet-stream') === true,
		),
		conditional:
			conditional.status === 304 &&
			(await conditional.arrayBuffer()).byteLength === 0 &&
			conditional.headers.get('Access-Control-Allow-Origin') === '*' &&
			conditional.headers.get('Cache-Control')?.includes('immutable') === true,
		missing: missing.status === 404 && missing.headers.get('Cache-Control') === 'no-store',
		aliasRedirect:
			redirect.status === 302 &&
			redirect.headers.get('Location') === '/0.1.0/runtime.js' &&
			redirect.headers.get('Access-Control-Allow-Origin') === '*' &&
			redirect.headers.get('Cache-Control') === 'no-store',
	};
	return { origin, checks, files, pass: Object.values(checks).every(Boolean) };
}
try {
	await Promise.all([
		startLocalVariant(assetsRoot, basePort),
		startLocalVariant(r2Root, basePort + 1),
	]);
	const [assets, r2Observation] = await Promise.all([
		probeVariant(basePort),
		probeVariant(basePort + 1),
	]);
	const result = {
		runtime: 'Wrangler 4.151.0 / local workerd',
		assets,
		r2Worker: r2Observation,
		limits:
			'No account, credentials, remote resource or deploy. Two named roots contain identical current build bytes. This proves local body/MIME/CORS/cache protocol, not deployed retention, alias updates, edge propagation, performance or cost.',
	};
	await writeFile(join(trialRoot, 'results.json'), JSON.stringify(result, null, 2), {
		mode: 0o600,
	});
	process.stdout.write(
		`${JSON.stringify({ assets: assets['checks'], r2Worker: r2Observation['checks'], resultDirectory: trialRoot }, null, 2)}\n`,
	);
	if (!assets['pass'] || !r2Observation['pass']) process.exitCode = 1;
	if (process.argv.includes('--serve')) {
		process.stdout.write(
			`Local-only runtimes remain on ${basePort}/${basePort + 1}; Ctrl-C stops this trial.\n`,
		);
		await new Promise<void>((finish): void => {
			process.once('SIGINT', finish);
			process.once('SIGTERM', finish);
		});
	}
} finally {
	for (const child of children) child.kill('SIGTERM');
}
