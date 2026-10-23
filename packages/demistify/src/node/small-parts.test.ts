import { expect, test } from 'vitest';

import { checkHtml, type Finding } from './check.ts';

const phase = (label: string, value: string, series: string): string =>
	`<data value="${value}" series="${series}">${label}</data>`;

async function findingsFor(content: string): Promise<Finding[]> {
	return checkHtml(
		`<!doctype html><html><head><link rel="stylesheet" href="/theme.css"><script type="module" src="/runtime.js"></script></head><body><ex-document>${content}</ex-document></body></html>`,
		{ diagrams: false },
	);
}

test('the checker accepts supported badge, progress and breakdown markup at their boundary states', async (): Promise<void> => {
	const findings = await findingsFor(`
		<p><ex-badge>Neutral label</ex-badge> <ex-badge tone="risk">Delayed</ex-badge></p>
		<ex-progress label="Checks" value="0" max="6"></ex-progress>
		<ex-progress label="Checks" value="2" max="6"></ex-progress>
		<ex-progress label="Checks" value="6" max="6"></ex-progress>
		<ex-progress label="Checks"></ex-progress>
		<ex-progress label="Checks" max="6"></ex-progress>
		<ex-progress label="Checks" value="0" max="0"></ex-progress>
		<ex-breakdown label="Request phases" unit="ms">${phase('Connect', '28', '1')}${phase('Work', '156', '2')}${phase('Transfer', '44', '3')}</ex-breakdown>
		<ex-breakdown label="Request phases" unit="ms" scale="elapsed" max="1240">${phase('Connect', '28', '1')}${phase('Work', '156', '2')}${phase('Transfer', '44', '3')}</ex-breakdown>
		<ex-breakdown label="Fractional phases" unit="s" scale="elapsed" max="0.3">${phase('Connect', '0.1', '1')}${phase('Work', '0.2', '2')}</ex-breakdown>
		<ex-breakdown label="Zero share" unit="ms">${phase('Connect', '0', '1')}</ex-breakdown>
		<ex-breakdown label="Zero elapsed" unit="ms" scale="elapsed" max="1240">${phase('Connect', '0', '1')}</ex-breakdown>
	`);
	expect(findings).toEqual([]);
});

test.each([
	{
		name: 'a value without a total',
		markup: '<ex-progress label="Checks" value="2"></ex-progress>',
		message: 'total',
	},
	{
		name: 'a fractional count',
		markup: '<ex-progress label="Checks" value="1.5" max="6"></ex-progress>',
		message: 'whole-number',
	},
	{
		name: 'a negative count',
		markup: '<ex-progress label="Checks" value="-1" max="6"></ex-progress>',
		message: 'negative',
	},
	{
		name: 'an inconsistent count',
		markup: '<ex-progress label="Checks" value="7" max="6"></ex-progress>',
		message: '7/6',
	},
])(
	'the checker reports $name with its exact authored value',
	async ({ markup, message }): Promise<void> => {
		const findings = await findingsFor(markup);
		const problem = findings.find((finding): boolean => finding.rule === 'small-parts');
		expect(problem).toMatchObject({ severity: 'error', rule: 'small-parts' });
		expect(problem?.message).toContain(message);
	},
);

test.each([
	{
		name: 'missing phase data',
		markup: '',
		message: 'one to three',
	},
	{
		name: 'a missing phase value',
		markup: '<data series="1">Connect</data>',
		message: 'value',
	},
	{
		name: 'a negative phase value',
		markup: phase('Connect', '-1', '1'),
		message: '-1',
	},
	{
		name: 'a non-finite phase value',
		markup: phase('Connect', 'Infinity', '1'),
		message: 'Infinity',
	},
	{
		name: 'a missing series',
		markup: '<data value="28">Connect</data>',
		message: 'series',
	},
	{
		name: 'an invalid series',
		markup: phase('Connect', '28', '4'),
		message: 'series',
	},
	{
		name: 'duplicate names',
		markup: `${phase('Connect', '28', '1')}${phase('Connect', '156', '2')}`,
		message: 'unique',
	},
	{
		name: 'duplicate slots',
		markup: `${phase('Connect', '28', '1')}${phase('Work', '156', '1')}`,
		message: 'unique',
	},
	{
		name: 'more than three phases',
		markup: `${phase('Connect', '1', '1')}${phase('Work', '2', '2')}${phase('Transfer', '3', '3')}${phase('Wait', '4', '4')}`,
		message: 'three',
	},
	{
		name: 'an unsupported scale',
		markup: `<ex-breakdown label="Request phases" unit="ms" scale="percent">${phase('Connect', '28', '1')}</ex-breakdown>`,
		message: 'scale',
	},
	{
		name: 'elapsed without max',
		markup: `<ex-breakdown label="Request phases" unit="ms" scale="elapsed">${phase('Connect', '28', '1')}</ex-breakdown>`,
		message: 'finite and positive',
	},
	{
		name: 'zero elapsed max even when all phases are zero',
		markup: `<ex-breakdown label="Request phases" unit="ms" scale="elapsed" max="0">${phase('Connect', '0', '1')}</ex-breakdown>`,
		message: 'finite and positive',
	},
	{
		name: 'elapsed total over max',
		markup: `<ex-breakdown label="Request phases" unit="ms" scale="elapsed" max="227">${phase('Connect', '28', '1')}${phase('Work', '156', '2')}${phase('Transfer', '44', '3')}</ex-breakdown>`,
		message: 'max="227"',
	},
	{
		name: 'fractional elapsed total just over max',
		markup: `<ex-breakdown label="Fractional phases" unit="s" scale="elapsed" max="0.3">${phase('Connect', '0.1', '1')}${phase('Work', '0.20000000000000004', '2')}</ex-breakdown>`,
		message: 'max="0.3"',
	},
	{
		name: 'fractional elapsed total clearly over max',
		markup: `<ex-breakdown label="Fractional phases" unit="s" scale="elapsed" max="0.3">${phase('Connect', '0.1', '1')}${phase('Work', '0.21', '2')}</ex-breakdown>`,
		message: 'max="0.3"',
	},
])(
	'the checker rejects $name through the shared parser',
	async ({ name, markup, message }): Promise<void> => {
		const content =
			name === 'missing phase data'
				? '<ex-breakdown label="Request phases" unit="ms"></ex-breakdown>'
				: markup.startsWith('<ex-breakdown')
					? markup
					: `<ex-breakdown label="Request phases" unit="ms">${markup}</ex-breakdown>`;
		const findings = await findingsFor(content);
		const problem = findings.find((finding): boolean => finding.rule === 'small-parts');
		expect(problem).toMatchObject({ severity: 'error', rule: 'small-parts' });
		expect(problem?.message.toLowerCase()).toContain(message.toLowerCase());
	},
);

test('the checker reports a phase outside the direct data-child contract', async (): Promise<void> => {
	const findings = await findingsFor(
		'<ex-breakdown label="Request phases" unit="ms"><p>Connect</p></ex-breakdown>',
	);
	const problem = findings.find(
		(finding): boolean => finding.rule === 'small-parts' && finding.message.includes('<data>'),
	);
	expect(problem).toMatchObject({ severity: 'error', rule: 'small-parts' });
});

test('the checker requires both labels on their owning parts', async (): Promise<void> => {
	const findings = await findingsFor(
		'<ex-progress value="2" max="6"></ex-progress><ex-breakdown label="Phases"></ex-breakdown>',
	);
	expect(findings.filter((finding): boolean => finding.rule === 'attribute')).toHaveLength(2);
});
