import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

import { recordSchema } from '@sunstack/demistify/design-records';
import { z } from 'zod';

/** A real authored collection with different entry identities and only its Help call path. */
export async function writeCollectionFixture(output: string): Promise<void> {
	const source = resolve(import.meta.dirname, '../experiments/understanding-lab');
	await mkdir(output, { recursive: true });
	await Promise.all(
		(await readdir(source))
			.filter((name) => name.endsWith('.html'))
			.map(async (file): Promise<void> => {
				let html = (await readFile(join(source, file), 'utf8')).replaceAll('U1', 'FirstNeed');
				if (file === 'execution.html') {
					const block = /(<script[^>]*data-ex-records[^>]*>)([\s\S]*?)(<\/script>)/.exec(html);
					if (!block?.[2]) throw new Error('Execution record block missing.');
					const records = z.array(recordSchema).parse(JSON.parse(block[2]));
					const stack = records.find(
						(record) => record.kind === 'stack' && record.id === 'open-help',
					);
					if (stack?.kind !== 'stack') throw new Error('Help stack missing.');
					const selected = records.filter(
						(record) => record.id === stack.id || stack.frames.includes(record.id),
					);
					html = html.replace(block[0], `${block[1]}${JSON.stringify(selected)}${block[3]}`);
					html = html.replaceAll('open-help', 'help-path');
				}
				await writeFile(join(output, file), html);
			}),
	);
}
