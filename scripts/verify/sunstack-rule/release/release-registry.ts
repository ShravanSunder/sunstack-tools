import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fileSystem from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import zlib from 'node:zlib';
import { z } from 'zod';
// S4 private registry: name resolution and URL-only repacks; no upstream proxy.







const packageMetadataSchema = z.looseObject({
	name: z.string(), version: z.string(), bin: z.looseObject({ sunstack: z.string() }),
	artifactDownloadUrls: z.json(),
});
const dependencyMetadataSchema = z.looseObject({ name: z.string(), version: z.string() });
const lockedDependencySchema = z.looseObject({ version: z.string(), integrity: z.string() });
const shrinkwrapSchema = z.object({ packages: z.looseObject({ 'node_modules/detect-libc': lockedDependencySchema }) });
const releaseDeclarationSchema = z.object({ older: z.object({ version: z.string() }), newer: z.object({ version: z.string() }) });
type PackageMetadata = z.infer<typeof packageMetadataSchema>;
interface ArchiveMember { readonly name: string; readonly kind: string; readonly header: Buffer; readonly data: Buffer; }
interface RepackProof {
	readonly original_tar_sha256: string; readonly adapted_tar_sha256: string;
	readonly original: PackageMetadata; readonly adapted: PackageMetadata;
	readonly before: Record<string, string>; readonly after: Record<string, string>; readonly changed: string[];
}
interface RepackedPackage { readonly tarball: Buffer; readonly metadata: PackageMetadata; readonly proof: RepackProof; }
interface RegistryRequest { readonly method: string | undefined; readonly path: string; readonly status: number; authorization_present?: boolean; sha256?: string; file?: string; }
interface RegistryVersion extends Record<string, unknown> { readonly dist: { readonly tarball: string; readonly integrity: string }; }
interface RegistryOptions { readonly assets: string; readonly declaration: unknown; readonly dependencyTar: string; readonly evidence: string; }
interface RegistryService { readonly base: string; readonly requests: RegistryRequest[]; readonly close: () => Promise<void>; }

const sha256 = (value: string | Buffer): string => crypto.createHash('sha256').update(value).digest('hex');
const integrity = (value: string | Buffer): string => 'sha512-' + crypto.createHash('sha512').update(value).digest('base64');
const save = (file: string, value: unknown): void => fileSystem.writeFileSync(file, JSON.stringify(value, null, 2) + '\n');

function unpackPackage(archive: Buffer): ArchiveMember[] {
	const bytes = zlib.gunzipSync(archive, { maxOutputLength: 16 * 1024 * 1024 });
	const members: ArchiveMember[] = [];
	for (let offset = 0; offset + 512 <= bytes.length;) {
		const header = bytes.subarray(offset, offset + 512);
		if (header.every((byte) => byte === 0)) break;
		const text = (start: number, length: number): string =>
			header
				.subarray(start, start + length)
				.toString()
				.replace(/\0.*$/s, '');
		const octal = (start: number, length: number): number => {
			const value = text(start, length).trim();
			assert.match(value, /^[0-7]+$/);
			return parseInt(value, 8);
		};
		const checksum = header.reduce(
			(sum, byte, index) => sum + (index >= 148 && index < 156 ? 32 : byte),
			0,
		);
		assert.equal(checksum, octal(148, 8), 'tar header checksum');
		const prefix = text(345, 155);
		const name = (prefix ? prefix + '/' : '') + text(0, 100);
		assert.ok(name.startsWith('package/') && /^[A-Za-z0-9_./-]+$/.test(name));
		assert.ok(!name.split('/').some((part) => part === '..' || part === '.'));
		assert.ok(!members.some((member) => member.name === name), 'duplicate package member');
		const kind = text(156, 1);
		assert.ok(kind === '' || kind === '0' || kind === '5', 'only regular files/directories');
		const size = octal(124, 12);
		assert.ok(size <= 1024 * 1024 && offset + 512 + size <= bytes.length);
		assert.ok(kind !== '5' || size === 0);
		members.push({
			name,
			kind,
			header: Buffer.from(header),
			data: Buffer.from(bytes.subarray(offset + 512, offset + 512 + size)),
		});
		assert.ok(members.length <= 100);
		offset += 512 + Math.ceil(size / 512) * 512;
	}
	assert.ok(members.length > 0);
	return members;
}

function repackPackage(archive: Buffer, version: string, mirror: string): RepackedPackage {
	const members = unpackPackage(archive);
	const metadata = members.find((member) => member.name === 'package/package.json');
	assert.ok(metadata);
	const before = packageMetadataSchema.parse(JSON.parse(metadata.data.toString('utf8')));
	assert.equal(before.name, '@sunstack/cli');
	assert.equal(before.version, version);
	assert.equal(before.bin.sunstack, 'run-sunstack.js');
	const after = { ...before, artifactDownloadUrls: [mirror] };
	assert.deepEqual({ ...after, artifactDownloadUrls: before.artifactDownloadUrls }, before);
	const chunks: Buffer[] = [];
	for (const member of members) {
		const data =
			member === metadata ? Buffer.from(JSON.stringify(after, null, 2) + '\n') : member.data;
		const header = Buffer.from(member.header);
		header.write(data.length.toString(8).padStart(11, '0') + '\0', 124, 12, 'ascii');
		header.fill(32, 148, 156);
		const checksum = header.reduce((sum, byte) => sum + byte, 0);
		header.write(checksum.toString(8).padStart(6, '0') + '\0 ', 148, 8, 'ascii');
		chunks.push(header, data, Buffer.alloc((512 - (data.length % 512)) % 512));
	}
	chunks.push(Buffer.alloc(1024));
	const repacked = zlib.gzipSync(Buffer.concat(chunks));
	const hashes = (list: readonly ArchiveMember[]): Record<string, string> =>
		Object.fromEntries(list.map((member) => [member.name, sha256(member.data)]));
	const originalHashes = hashes(members),
		adaptedHashes = hashes(unpackPackage(repacked));
	assert.deepEqual(Object.keys(originalHashes), Object.keys(adaptedHashes));
	const changed = Object.keys(originalHashes).filter(
		(name) => originalHashes[name] !== adaptedHashes[name],
	);
	assert.deepEqual(changed, ['package/package.json']);
	return {
		tarball: repacked,
		metadata: after,
		proof: {
			original_tar_sha256: sha256(archive),
			adapted_tar_sha256: sha256(repacked),
			original: before,
			adapted: after,
			before: originalHashes,
			after: adaptedHashes,
			changed,
		},
	};
}

async function startRegistry({ assets, declaration: rawDeclaration, dependencyTar, evidence }: RegistryOptions): Promise<RegistryService> {
	const declaration = releaseDeclarationSchema.parse(rawDeclaration);
	fileSystem.mkdirSync(evidence, { recursive: true });
	const requests: RegistryRequest[] = [];
	const packages = new Map<string, Record<string, RegistryVersion>>();
	const tarballs = new Map<string, Buffer>();
	const files = new Map<string, string>();
	const server = http.createServer((request, response) => {
		let pathname: string;
		try {
			pathname = decodeURIComponent(new URL(z.string().parse(request.url), base).pathname);
		} catch {
			response.writeHead(400).end();
			return;
		}
		let body: Buffer | undefined,
			file: string | undefined,
			contentType = 'application/json';
		if (!['GET', 'HEAD'].includes(request.method ?? '') || request.headers.authorization) {
			requests.push({
				method: request.method,
				path: pathname,
				status: 405,
				authorization_present: Boolean(request.headers.authorization),
			});
			response.writeHead(405).end();
			return;
		}
		if (pathname === '/-/ping') body = Buffer.from('{}');
		else if (tarballs.has(pathname)) {
			body = tarballs.get(pathname);
			contentType = 'application/octet-stream';
		} else if (files.has(pathname)) {
			file = files.get(pathname);
			contentType = 'application/octet-stream';
		} else {
			for (const [name, versions] of packages) {
				if (pathname === '/' + name) {
					body = Buffer.from(
						JSON.stringify({
							name,
							'dist-tags': { latest: Object.keys(versions).at(-1) },
							versions,
						}),
					);
					break;
				}
				const version = pathname.startsWith('/' + name + '/')
					? pathname.slice(name.length + 2)
					: null;
				if (version && versions[version]) {
					body = Buffer.from(JSON.stringify(versions[version]));
					break;
				}
			}
		}
		const status = body || file ? 200 : 404;
		const record: RegistryRequest = { method: request.method, path: pathname, status };
		if (body) record.sha256 = sha256(body);
		if (file) record.file = file;
		requests.push(record);
		response.writeHead(status, { 'content-type': contentType, 'cache-control': 'no-store' });
		if (request.method === 'HEAD') response.end();
		else if (file) fileSystem.createReadStream(file).pipe(response);
		else response.end(body || '{}');
	});
	await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
	const address = server.address();
	assert.ok(address && typeof address === 'object');
	const base = 'http://127.0.0.1:' + address.port;
	try {
		const cliVersions: Record<string, RegistryVersion> = {};
		let lockedDependency: z.infer<typeof lockedDependencySchema> | undefined;
		const labels: readonly ('older' | 'newer')[] = ['older', 'newer'];
		for (const label of labels) {
			const version = declaration[label].version;
			const archivePath = path.join(assets, label, 'sunstack-npm-package.tar.gz');
			const archive = fileSystem.readFileSync(archivePath);
			const sums = fileSystem
				.readFileSync(path.join(assets, label, 'sha256.sum'), 'utf8')
				.split('\n')
				.filter((line) => line.trim());
			const npmSum = sums
				.map((line) => line.trim().split(/\s+/))
				.filter((fields) => fields[1]?.replace(/^\*/, '') === 'sunstack-npm-package.tar.gz');
			assert.equal(npmSum.length, 1, 'one authenticated unified npm checksum required');
			const expectedSum = npmSum[0];
			assert.ok(expectedSum);
			assert.equal(sha256(archive), expectedSum[0]);
			const original = unpackPackage(archive);
			const lockMember = original.find((member) => member.name === 'package/npm-shrinkwrap.json');
			assert.ok(lockMember);
			const lock = shrinkwrapSchema.parse(JSON.parse(lockMember.data.toString('utf8')));
			const required = lock.packages['node_modules/detect-libc'];
			assert.equal(required.version, '2.1.2');
			if (lockedDependency) assert.deepEqual(required, lockedDependency);
			else lockedDependency = required;
			const prepared = repackPackage(archive, version, base + '/archives/' + label);
			save(path.join(evidence, label + '-url-only.json'), prepared.proof);
			fileSystem.writeFileSync(path.join(evidence, label + '-adapted.tgz'), prepared.tarball);
			const route = '/tarballs/sunstack-cli-' + version + '.tgz';
			tarballs.set(route, prepared.tarball);
			cliVersions[version] = {
				...prepared.metadata,
				dist: { tarball: base + route, integrity: integrity(prepared.tarball) },
			};
			for (const filename of fileSystem.readdirSync(path.join(assets, label))) {
				if (!/^sunstack[-.][A-Za-z0-9_.-]+$/.test(filename)) continue;
				const file = path.join(assets, label, filename);
				if (fileSystem.statSync(file).isFile())
					files.set('/archives/' + label + '/' + filename, file);
			}
		}
		packages.set('@sunstack/cli', cliVersions);
		const dependency = fileSystem.readFileSync(dependencyTar);
		assert.ok(lockedDependency);
		assert.equal(integrity(dependency), lockedDependency.integrity);
		const dependencyMember = unpackPackage(dependency).find((member) => member.name === 'package/package.json');
		assert.ok(dependencyMember);
		const dependencyMetadata = dependencyMetadataSchema.parse(JSON.parse(dependencyMember.data.toString('utf8')));
		assert.equal(dependencyMetadata.name, 'detect-libc');
		assert.equal(dependencyMetadata.version, '2.1.2');
		const route = '/tarballs/detect-libc-2.1.2.tgz';
		tarballs.set(route, dependency);
		packages.set('detect-libc', {
			'2.1.2': {
				...dependencyMetadata,
				dist: { tarball: base + route, integrity: integrity(dependency) },
			},
		});
		save(path.join(evidence, 'registry.json'), {
			pid: process.pid,
			bind: '127.0.0.1',
			base,
			package_versions: [...packages].map(([name, versions]) => ({
				name,
				versions: Object.keys(versions),
			})),
			upstream: false,
		});
		return {
			base,
			requests,
			close: async (): Promise<void> => {
				server.closeIdleConnections();
				await new Promise<void>((resolve) => server.close(() => resolve()));
				save(path.join(evidence, 'requests.json'), requests);
				save(path.join(evidence, 'cleanup.json'), { listening: server.listening });
			},
		};
	} catch (error) {
		server.closeAllConnections();
		await new Promise<void>((resolve) => server.close(() => resolve()));
		throw error;
	}
}

export { unpackPackage, repackPackage, startRegistry, sha256, integrity, save };
