// Formula-specific private-tap installation and owned uninstall lifecycle.
import assert from 'node:assert/strict';
import fileSystem from 'node:fs';
import path from 'node:path';

import { provenanceSchema, type RawCommand } from './release-records.ts';
interface HomebrewOptions {
	assets: string;
	label: string;
	version: string;
	cell: string;
	environment: NodeJS.ProcessEnv;
	mirrorBase: string;
	raw: RawCommand;
}
import { captureProcess } from './release-process.ts';
import { sha256, save } from './release-registry.ts';

async function installHomebrew({
	assets,
	label,
	version,
	cell,
	environment,
	mirrorBase,
	raw,
}: HomebrewOptions): Promise<{ binary: string; cleanup: () => Promise<void> }> {
	const brew =
		process.env['S4_BREW'] && process.env['S4_BREW'].length > 0
			? process.env['S4_BREW']
			: process.platform === 'darwin'
				? '/opt/homebrew/bin/brew'
				: '/home/linuxbrew/.linuxbrew/bin/brew';
	const env = {
		...environment,
		PATH: path.dirname(brew) + ':' + environment['PATH'],
		HOMEBREW_NO_AUTO_UPDATE: '1',
		HOMEBREW_NO_ANALYTICS: '1',
		HOMEBREW_NO_INSTALL_CLEANUP: '1',
	};
	let installOwned = false;
	async function cleanup(): Promise<void> {
		if (!installOwned) return;
		const listing = await captureProcess({
			command: [brew, 'list', '--versions', 'sunstack'],
			cwd: cell,
			environment: env,
			stem: path.join(cell, 'cleanup-list'),
		});
		assert.equal(listing.observation.error, null);
		assert.ok(
			listing.observation.root_close?.code === 0 || listing.observation.root_close?.code === 1,
		);
		if (listing.stdout.toString().trim())
			await raw(
				[brew, 'uninstall', '--formula', 'sunstack'],
				cell,
				env,
				path.join(cell, 'uninstall'),
			);
		installOwned = false;
	}
	try {
		const prior = await captureProcess({
			command: [brew, 'list', '--versions', 'sunstack'],
			cwd: cell,
			environment: env,
			stem: path.join(cell, 'preexisting'),
		});
		assert.equal(prior.observation.error, null);
		assert.ok(prior.observation.root_close?.code === 0 || prior.observation.root_close?.code === 1);
		assert.equal(prior.stdout.toString().trim(), '', 'refuse preexisting Homebrew sunstack');
		const original = fileSystem.readFileSync(
			path.join(assets, label, 'ci-pushed-sunstack.rb'),
			'utf8',
		);
		const provenance = provenanceSchema.parse(
			JSON.parse(fileSystem.readFileSync(path.join(assets, label, 'tap-provenance.json'), 'utf8')),
		);
		assert.equal(provenance.repository, 'ShravanSunder/homebrew-sunstack-rehearsal');
		assert.equal(provenance.version, version);
		assert.equal(sha256(Buffer.from(original)), provenance.formula_sha256);
		const sourceBase =
			'https://github.com/ShravanSunder/sunstack-release-rehearsal/releases/download/v' + version;
		assert.ok(original.includes(sourceBase));
		const replacement = mirrorBase + '/archives/' + label;
		const adapted = original.replaceAll(sourceBase, replacement);
		assert.equal(adapted.replaceAll(replacement, sourceBase), original);
		save(path.join(cell, 'formula-provenance.json'), provenance);
		fileSystem.writeFileSync(path.join(cell, 'formula-original.rb'), original);
		fileSystem.writeFileSync(path.join(cell, 'formula-adapted.rb'), adapted);
		const tap = 'verifier/s4-v5-' + label;
		await raw([brew, 'tap-new', '--no-git', tap], cell, env, path.join(cell, 'tap-new'));
		const tapRoot = (
			await raw([brew, '--repo', tap], cell, env, path.join(cell, 'tap-root'))
		).stdout
			.toString()
			.trim();
		const formula = path.join(tapRoot, 'Formula/sunstack.rb');
		fileSystem.mkdirSync(path.dirname(formula), { recursive: true });
		fileSystem.writeFileSync(formula, adapted);
		await raw(
			[brew, 'trust', '--formula', tap + '/sunstack'],
			cell,
			env,
			path.join(cell, 'trust-one-formula'),
		);
		installOwned = true;
		await raw([brew, 'install', '--formula', formula], cell, env, path.join(cell, 'install'));
		const prefix = (
			await raw([brew, '--prefix', 'sunstack'], cell, env, path.join(cell, 'prefix'))
		).stdout
			.toString()
			.trim();
		return { binary: path.join(prefix, 'bin/sunstack'), cleanup };
	} catch (error) {
		try {
			await cleanup();
		} catch (cleanupError) {
			throw new AggregateError([error, cleanupError], 'Homebrew install and owned cleanup failed', {
				cause: cleanupError,
			});
		}
		throw error;
	}
}
export { installHomebrew };
