import { join, resolve } from 'node:path';

import { expect, it } from 'vitest';

import { designRecordsFilename, type DesignRecordView } from '../../core/design-record-view.ts';
import { readPageMarkup } from './design-page-markup.ts';
import { recordOutputFile } from './record-output.ts';

const site = resolve('site');
const milestone = join(site, 'project', 'milestone');
const page = join(milestone, 'index.html');
const navOutput = join(milestone, designRecordsFilename);
const projectOutput = join(site, 'project', designRecordsFilename);
const outputs = new Map<string, DesignRecordView>([
	[navOutput, { version: 1, pages: {} }],
	[projectOutput, { version: 1, pages: {} }],
]);
const nav = '<ex-nav src="navigation.generated.json"></ex-nav>';
const overview = '<ex-project-overview src="../project.generated.json"></ex-project-overview>';
const marker = '<meta name="demistify-records" content="../records.generated.js">';

it.each([
	{
		name: 'navigation with two overviews and two markers',
		html: nav + overview + overview + marker + marker,
		expected: navOutput,
	},
	{
		name: 'overview with two markers and static navigation',
		html: overview + marker + marker + '<ex-nav src="navigation.json"></ex-nav>',
		expected: projectOutput,
	},
	{
		name: 'encoded navigation before a project overview',
		html: nav.replace('navigation.generated.json', './%6eavigation.generated.json') + overview,
		expected: navOutput,
	},
	{ name: 'the marker as fallback', html: marker, expected: projectOutput },
	{
		name: 'a marker with repeated separators',
		html: marker.replace('../records.generated.js', '..//records.generated.js'),
		expected: projectOutput,
	},
	{
		name: 'navigation with an invalid lower-priority hint',
		html: nav + overview.replace('project.generated.json', '%GG/project.generated.json'),
		expected: navOutput,
	},
	{
		name: 'an encoded marker as fallback',
		html: marker.replace('records.generated.js', '%72ecords.generated.js'),
		expected: projectOutput,
	},
])('selects $name', ({ html, expected }): void => {
	expect(recordOutputFile(readPageMarkup(html, page), page, site, outputs)).toBe(expected);
});

it.each([
	{
		name: 'navigation',
		html: nav + nav + overview,
		error: /multiple generated navigation sources/,
	},
	{
		name: 'overview',
		html: overview + overview + marker,
		error: /multiple generated overview sources/,
	},
	{ name: 'marker', html: marker + marker, error: /provide only one demistify-records marker/ },
])('rejects a tie at the selected $name priority', ({ html, error }): void => {
	expect(() => recordOutputFile(readPageMarkup(html, page), page, site, outputs)).toThrow(error);
});
