/** Independently fetch pinned GitHub rendered IDs for twelve found links and all five misses. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { runSequence, writeJson, type JsonObject } from '../rulebook/rulebook-fixture-support.ts';
import { repositories, type Link } from './oracles.ts';

export async function observeGitHub(evidence: string): Promise<void> {
	filesystem.mkdirSync(evidence);
	const checks: JsonObject[] = [];
	await runSequence(repositories, async (repository) => {
		const seen = new Set<string>();
		const found: Link[] = [];
		for (const link of repository.links) {
			const key = `${link.target}#${link.fragment}`;
			if (link.expected === 'found' && !seen.has(key) && found.length < 6) {
				found.push(link);
				seen.add(key);
			}
		}
		assert.equal(found.length, 6);
		const selected = [...found, ...repository.links.filter((link) => link.expected !== 'found')];
		const pages = new Map<string, { status: number; html: string; receipt: string }>();
		await runSequence(selected, async (link) => {
			let page = pages.get(link.target);
			if (!page) {
				const url = `https://api.github.com/repos/${repository.repo}/contents/${link.target.split('/').map(encodeURIComponent).join('/')}?ref=${repository.commit}`;
				const response = await fetch(url, {
					headers: {
						Accept: 'application/vnd.github.html+json',
						'User-Agent': 'sunstack-verifier',
					},
					signal: AbortSignal.timeout(30000),
				});
				const html = await response.text();
				assert.ok(Buffer.byteLength(html) < 10_000_000, 'Rendered page exceeded bounded capture');
				const name = `${repository.key}-${createHash('sha256').update(link.target).digest('hex').slice(0, 12)}`;
				filesystem.writeFileSync(path.join(evidence, `${name}.html`), html);
				const receipt = `${name}.receipt.json`;
				writeJson(path.join(evidence, receipt), {
					url,
					status: response.status,
					headers: Object.fromEntries(response.headers),
					sha256: createHash('sha256').update(html).digest('hex'),
				});
				page = { status: response.status, html, receipt };
				pages.set(link.target, page);
			}
			if (page.status === 200)
				assert.match(
					page.html,
					/class="markdown-body\b/,
					'Response is not a rendered Markdown article',
				);
			const ids = [...page.html.matchAll(/\bid="user-content-([^"]*)"/g)].map((match) => match[1]);
			const present = ids.includes(link.fragment);
			const passed =
				link.expected === 'missing_file'
					? page.status === 404
					: page.status === 200 && present === (link.expected === 'found');
			checks.push({
				repo: repository.repo,
				source: link.source,
				target: link.target,
				fragment: link.fragment,
				expected: link.expected,
				http: page.status,
				rendered_present: present,
				receipt: page.receipt,
				passed,
			});
			writeJson(path.join(evidence, 'assessment.json'), {
				checks,
				all_passed: checks.every((check) => check['passed'] === true),
			});
			assert.ok(passed, 'Pinned link differs on actual GitHub rendered page; return to Lead');
		});
	});
}
if (process.argv[1] === import.meta.filename) {
	const { values } = parseArgs({
		options: { evidence: { type: 'string' }, help: { type: 'boolean' } },
	});
	if (values.help)
		process.stdout.write(
			'Fetch pinned public GitHub rendered HTML without credentials; preserve IDs/HTTP receipts for 12 found / 4 absent / 1 file 404.\n--evidence NEW_DIRECTORY\nRequires explicit keyless read-only network grant. Does not drive Sunstack.\n',
		);
	else await observeGitHub(z.string().parse(values.evidence));
}
