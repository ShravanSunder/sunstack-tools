#!/usr/bin/env node
// Keep cargo-dist installation; async execution preserves native cancellation and exits.
const { spawn } = require('node:child_process');
const { join } = require('node:path');
const { getPackage } = require('./binary');

async function runSunstack() {
	const binaryPackage = getPackage();
	if (!binaryPackage.exists()) await binaryPackage.install(true);
	const child = spawn(
		join(binaryPackage.installDirectory, binaryPackage.binaries.sunstack),
		process.argv.slice(2),
		{ stdio: 'inherit' },
	);
	/** @type {NodeJS.Signals[]} */
	const signals = ['SIGINT', 'SIGTERM', 'SIGHUP'];
	const handlers = new Map(signals.map((signal) => [signal, () => child.kill(signal)]));
	for (const [signal, handler] of handlers) process.on(signal, handler);
	child.on('error', (error) => {
		process.stderr.write(`${error.message}\n`);
		process.exitCode = 1;
	});
	child.on('close', (code, signal) => {
		for (const [forwarded, handler] of handlers) process.removeListener(forwarded, handler);
		if (signal) process.kill(process.pid, signal);
		else process.exitCode = code ?? 1;
	});
}

runSunstack().catch((error) => {
	process.stderr.write(`${error.message}\n`);
	process.exitCode = 1;
});
