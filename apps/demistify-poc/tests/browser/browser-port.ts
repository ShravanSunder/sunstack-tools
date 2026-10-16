import { createServer } from 'node:net';

/** Vitest 5 treats API port 0 as its fixed default, so choose a free loopback port explicitly. */
export async function browserPort(): Promise<number> {
	const reservation = createServer();
	await new Promise<void>((resolve, reject): void => {
		reservation.once('error', reject);
		reservation.listen(0, '127.0.0.1', resolve);
	});
	const address = reservation.address();
	if (!address || typeof address === 'string') throw new Error('No browser test port allocated.');
	await new Promise<void>((resolve, reject): void => {
		reservation.close((error): void => {
			if (error) reject(error);
			else resolve();
		});
	});
	return address.port;
}
