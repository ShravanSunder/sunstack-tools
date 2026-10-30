#!/usr/bin/env node
// Keep cargo-dist installation; async execution preserves native cancellation and exits.
import { spawn } from 'node:child_process';
import { join } from 'node:path';

import { getPackage } from './binary.js';

async function runSunstack(): Promise<void> {
	const binaryPackage = getPackage();
	if (!binaryPackage.exists()) await binaryPackage.install(true);
	const child = spawn(
		join(binaryPackage.installDirectory, binaryPackage.binaries.sunstack),
		process.argv.slice(2),
		{ stdio: 'inherit' },
	);
	const signals: NodeJS.Signals[] = ['SIGINT', 'SIGTERM', 'SIGHUP'];
	const handlers = new Map(
		signals.map((signal) => [
			signal,
			(): void => {
				child.kill(signal);
			},
		]),
	);
	for (const [signal, handler] of handlers) process.on(signal, handler);
	child.on('error', (error): void => {
		process.stderr.write(`${error.message}\n`);
		process.exitCode = 1;
	});
	child.on('close', (code, signal): void => {
		for (const [forwarded, handler] of handlers) process.removeListener(forwarded, handler);
		if (signal) process.kill(process.pid, signal);
		else process.exitCode = code ?? 1;
	});
}

void runSunstack().catch((error: unknown): void => {
	process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
	process.exitCode = 1;
});
