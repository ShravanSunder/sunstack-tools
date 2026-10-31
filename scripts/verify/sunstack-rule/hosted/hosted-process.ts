/** Bounded owned child capture and the static arm64 ELF gate for hosted observation. */
import { spawn } from 'node:child_process';

import { requireCondition } from '../rulebook/rulebook-fixture-support.ts';

export interface CommandResult {
	stdout: Buffer;
	stderr: Buffer;
	exit: number | null;
	timedOut: boolean;
}
export async function runCommand(options: {
	executable: string;
	arguments: string[];
	environment?: NodeJS.ProcessEnv;
	timeout: number;
}): Promise<CommandResult> {
	const child = spawn(options.executable, options.arguments, {
		env: options.environment,
		stdio: ['ignore', 'pipe', 'pipe'],
	});
	const stdout: Buffer[] = [];
	const stderr: Buffer[] = [];
	child.stdout.on('data', (chunk: Buffer) => stdout.push(chunk));
	child.stderr.on('data', (chunk: Buffer) => stderr.push(chunk));
	let timedOut = false;
	const timer = setTimeout(() => {
		timedOut = true;
		child.kill('SIGKILL');
	}, options.timeout);
	try {
		const exit = await new Promise<number | null>((resolve, reject) => {
			child.once('error', reject);
			child.once('close', (code) => resolve(code));
		});
		return { stdout: Buffer.concat(stdout), stderr: Buffer.concat(stderr), exit, timedOut };
	} finally {
		clearTimeout(timer);
	}
}
export function inspectStaticArm64(payload: Buffer): void {
	requireCondition(
		payload.length >= 64 && payload.subarray(0, 6).equals(Buffer.from([127, 69, 76, 70, 2, 1])),
		'Expected ELF64 little-endian binary',
	);
	requireCondition(payload.readUInt16LE(18) === 183, 'Expected arm64 ELF machine');
	const offset = Number(payload.readBigUInt64LE(32));
	const size = payload.readUInt16LE(54);
	const count = payload.readUInt16LE(56);
	requireCondition(
		Number.isSafeInteger(offset) &&
			size >= 56 &&
			count > 0 &&
			offset + size * count <= payload.length,
		'ELF program headers out of bounds',
	);
	for (let index = 0; index < count; index++) {
		const header = offset + size * index;
		const kind = payload.readUInt32LE(header);
		requireCondition(kind !== 3, 'ELF requires dynamic interpreter');
		const start = Number(payload.readBigUInt64LE(header + 8));
		const length = Number(payload.readBigUInt64LE(header + 32));
		requireCondition(
			Number.isSafeInteger(start) &&
				Number.isSafeInteger(length) &&
				start + length <= payload.length,
			'ELF segment out of bounds',
		);
		if (kind === 2)
			for (let entry = start; entry + 16 <= start + length; entry += 16) {
				const tag = payload.readBigInt64LE(entry);
				requireCondition(tag !== 1n, 'ELF requires shared libraries');
				if (tag === 0n) break;
			}
	}
}
