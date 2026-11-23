import { dirname, join } from 'node:path';

import {
	designRecordsFilename,
	designRecordViewSchema,
	type DesignRecordView,
} from '../../core/design-record-view.ts';
import { resolveDesignLink } from './design-location.ts';
import type { PageMarkup } from './design-page-markup.ts';
import { relativePage } from './record-projection.ts';

export function recordOutputFile(
	markup: PageMarkup,
	file: string,
	siteRoot: string,
	outputs: ReadonlyMap<string, DesignRecordView>,
): string {
	if (markup.hasDocumentBase)
		throw new Error(`${file}: remove <base href> so design links resolve from the authored page.`);
	for (const [kind, sources, name] of [
		['navigation', markup.navigationSources, 'navigation.generated.json'],
		['overview', markup.projectOverviewSources, 'project.generated.json'],
	] as const) {
		const candidates = sources.filter((href) => href.split('/').at(-1) === name);
		if (candidates.length > 1)
			throw new Error(`${file}: multiple generated ${kind} sources for record bindings.`);
		const [binding] = candidates;
		if (!binding) continue;
		const target = resolveDesignLink(binding, file, siteRoot);
		if (!target) throw new Error('Generated records require a local source in ' + file + '.');
		return checkedOutput(join(dirname(target.file), designRecordsFilename));
	}
	if (markup.recordSourceMarkers.length > 1)
		throw new Error(`${file}: provide only one demistify-records marker.`);
	const [marker] = markup.recordSourceMarkers;
	if (!marker?.trim())
		throw new Error(
			`${file}: record bindings need a generated navigation or overview source, or <meta name="demistify-records" content="../milestone/${designRecordsFilename}">.`,
		);
	const target = resolveDesignLink(marker.trim(), file, siteRoot);
	if (!target || /[?#]/u.test(marker))
		throw new Error(`${file}: demistify-records must name a relative ${designRecordsFilename}.`);
	return checkedOutput(target.file);

	function checkedOutput(path: string): string {
		if (!outputs.has(path))
			throw new Error(
				`${file}: records source must name ${designRecordsFilename} in a registered project or design output folder.`,
			);
		return path;
	}
}

export function putPageRecords(
	view: DesignRecordView,
	outputFile: string,
	page: string,
	records: DesignRecordView['pages'][string],
): void {
	view.pages[relativePage(dirname(outputFile), page)] = records;
}

export function recordRegistrationScript(view: DesignRecordView): string {
	return `document.currentScript.dataset.demistifyRecords = ${JSON.stringify(JSON.stringify(designRecordViewSchema.parse(view)))};\n`;
}
