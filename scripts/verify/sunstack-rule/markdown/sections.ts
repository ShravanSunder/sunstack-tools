/** Observe real Markdown section fields and explicit container/admission controls. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import { writeProject } from '../html-evidence/html-fixtures.ts';
import { askingRule, sectionFields } from '../html-sections/fixtures.ts';
import { listingPieces } from '../rulebook/rulebook-evidence-checks.ts';
import {
	objectValue,
	readObject,
	writeJson,
	runSequence,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import type { RulebookProcess } from '../rulebook/rulebook-process.ts';
import { requireSections } from './checks.ts';
import { requireFile, sectionFiles } from './oracles.ts';

export async function observeSections(
	observer: RulebookProcess,
	input: string,
	evidence: string,
): Promise<number> {
	let total = 0;
	await runSequence([...sectionFiles.entries()], async ([index, file]) => {
		const raw = requireFile(path.join(input, file.repo), file);
		const root = path.join(evidence, `sections-${index}`);
		filesystem.mkdirSync(root);
		filesystem.writeFileSync(path.join(root, 'input.md'), raw);
		writeProject(
			root,
			{ K001: askingRule },
			{
				section: {
					markdown_units: { mode: 'unit', units: ['section'] },
					fields: sectionFields,
					max_chars: 300_000,
				},
			},
			['input.md'],
			'id: unused\nlanguage: Markdown\nrule: {kind: document}\n',
		);
		const configFile = path.join(root, 'sunstack.lint.json');
		const config = readObject(configFile);
		objectValue(config['decision_model'])['endpoint'] = observer.endpoint;
		writeJson(configFile, config);
		await observer.run({
			root,
			name: `sections-check-${index}`,
			arguments: ['lint', '--check'],
			expectedExit: 0,
		});
		const result = await observer.run({
			root,
			name: `sections-outline-${index}`,
			arguments: ['lint', '--all', '--evidence', 'section'],
			expectedExit: 0,
		});
		requireSections(listingPieces(result.stdout), file.sections, raw);
		total += file.sections.length;
	});
	const root = path.join(evidence, 'sections-3');
	const configFile = path.join(root, 'sunstack.lint.json');
	const original = readObject(configFile);
	const ordinaryFields = {
		full: { from: 'remainder' },
		own: { from: 'remainder' },
		parent: { from: 'remainder' },
	};
	const variants: readonly {
		name: string;
		matcher: JsonObject;
		fields: JsonObject;
		exit: number;
	}[] = [
		{
			name: 'mixed-ordinary',
			matcher: { mode: 'unit', units: ['section', 'paragraph'] },
			fields: ordinaryFields,
			exit: 0,
		},
		{
			name: 'paragraph-ordinary',
			matcher: { mode: 'unit', units: ['paragraph'] },
			fields: ordinaryFields,
			exit: 0,
		},
		{
			name: 'mixed-section-refusal',
			matcher: { mode: 'unit', units: ['section', 'paragraph'] },
			fields: sectionFields,
			exit: 2,
		},
		{
			name: 'paragraph-section-refusal',
			matcher: { mode: 'unit', units: ['paragraph'] },
			fields: sectionFields,
			exit: 2,
		},
		{
			name: 'call-site-refusal',
			matcher: { mode: 'call_site', units: ['section'], starts_with: 'Root' },
			fields: sectionFields,
			exit: 2,
		},
	];
	await runSequence(variants, async (variant) => {
		const config = structuredClone(original);
		objectValue(config['evidence'])['kinds'] = {
			section: { markdown_units: variant.matcher, fields: variant.fields },
		};
		writeJson(configFile, config);
		const result = await observer.run({
			root,
			name: variant.name,
			arguments: ['lint', '--check'],
			expectedExit: variant.exit,
		});
		if (variant.exit !== 0)
			assert.match(
				result.stderr.toString(),
				/markdown|section|unit/i,
				'Admission refusal must name its reason',
			);
	});
	writeJson(configFile, original);
	return total;
}
