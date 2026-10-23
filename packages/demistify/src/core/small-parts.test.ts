import { expect, test } from 'vitest';

import { parseBreakdownValue, parseProgressValue, type BreakdownPartInput } from './small-parts.ts';

const phases: readonly BreakdownPartInput[] = [
	{ label: 'Connect', value: '28', series: '1' },
	{ label: 'Work', value: '156', series: '2' },
	{ label: 'Transfer', value: '44', series: '3' },
];

test.each([
	{ name: 'no value and no total', value: null, max: null, expected: { kind: 'unknown' } },
	{ name: 'no value with a supplied total', value: null, max: '6', expected: { kind: 'unknown' } },
	{
		name: 'zero of six',
		value: '0',
		max: '6',
		expected: { kind: 'known', value: 0, max: 6 },
	},
	{
		name: 'two of six',
		value: '2',
		max: '6',
		expected: { kind: 'known', value: 2, max: 6 },
	},
	{
		name: 'six of six',
		value: '6',
		max: '6',
		expected: { kind: 'known', value: 6, max: 6 },
	},
	{ name: 'empty total', value: '0', max: '0', expected: { kind: 'empty', value: 0, max: 0 } },
	{
		name: 'value without total',
		value: '2',
		max: null,
		expected: { kind: 'invalid', reason: 'missing-total' },
	},
	{
		name: 'fractional count',
		value: '1.5',
		max: '6',
		expected: { kind: 'invalid', reason: 'not-integer' },
	},
	{
		name: 'non-finite count',
		value: 'Infinity',
		max: '6',
		expected: { kind: 'invalid', reason: 'not-integer' },
	},
	{
		name: 'negative count',
		value: '-1',
		max: '6',
		expected: { kind: 'invalid', reason: 'negative' },
	},
	{
		name: 'negative total',
		value: '0',
		max: '-1',
		expected: { kind: 'invalid', reason: 'negative' },
	},
	{
		name: 'count greater than total',
		value: '7',
		max: '6',
		expected: { kind: 'invalid', reason: 'exceeds-total' },
	},
])('progress parser classifies $name without guessing', ({ value, max, expected }): void => {
	expect(parseProgressValue(value, max)).toEqual(expected);
});

test('share breakdown preserves exact parts and uses their sum as its denominator', (): void => {
	expect(parseBreakdownValue(phases, null, null)).toEqual({
		kind: 'known',
		parts: [
			{ label: 'Connect', value: 28, series: 1 },
			{ label: 'Work', value: 156, series: 2 },
			{ label: 'Transfer', value: 44, series: 3 },
		],
		total: 228,
		max: 228,
		scale: 'share',
	});
});

test('elapsed breakdown uses its authored common maximum', (): void => {
	expect(parseBreakdownValue(phases, 'elapsed', '1240')).toMatchObject({
		kind: 'known',
		total: 228,
		max: 1240,
		scale: 'elapsed',
	});
});

test('elapsed breakdown accepts a decimal phase sum equal to its maximum', (): void => {
	expect(
		parseBreakdownValue(
			[
				{ label: 'Connect', value: '0.1', series: '1' },
				{ label: 'Work', value: '0.2', series: '2' },
			],
			'elapsed',
			'0.3',
		),
	).toEqual({
		kind: 'known',
		parts: [
			{ label: 'Connect', value: 0.1, series: 1 },
			{ label: 'Work', value: 0.2, series: 2 },
		],
		total: 0.3,
		max: 0.3,
		scale: 'elapsed',
	});
});

test.each([
	{
		name: 'zero share',
		scale: null,
		max: null,
		expected: { kind: 'zero', total: 0, max: 0, scale: 'share' },
	},
	{
		name: 'zero elapsed with a common maximum',
		scale: 'elapsed',
		max: '1240',
		expected: { kind: 'zero', total: 0, max: 1240, scale: 'elapsed' },
	},
])('zero breakdown retains its $name scale', ({ scale, max, expected }): void => {
	const zeroParts: readonly BreakdownPartInput[] = phases.map((part) => ({ ...part, value: '0' }));
	expect(parseBreakdownValue(zeroParts, scale, max)).toMatchObject(expected);
});

test.each([
	{
		name: 'missing phases',
		parts: [],
		scale: null,
		max: null,
		reason: 'missing-parts',
	},
	{
		name: 'an empty phase name',
		parts: [{ label: '  ', value: '4', series: '1' }],
		scale: null,
		max: null,
		reason: 'missing-parts',
	},
	{
		name: 'a missing phase value',
		parts: [{ label: 'Connect', value: null, series: '1' }],
		scale: null,
		max: null,
		reason: 'invalid-value',
	},
	{
		name: 'a malformed phase value',
		parts: [{ label: 'Connect', value: 'many', series: '1' }],
		scale: null,
		max: null,
		reason: 'invalid-value',
	},
	{
		name: 'a non-finite phase value',
		parts: [{ label: 'Connect', value: 'NaN', series: '1' }],
		scale: null,
		max: null,
		reason: 'invalid-value',
	},
	{
		name: 'a negative phase value',
		parts: [{ label: 'Connect', value: '-1', series: '1' }],
		scale: null,
		max: null,
		reason: 'invalid-value',
	},
	{
		name: 'a missing palette slot',
		parts: [{ label: 'Connect', value: '4', series: null }],
		scale: null,
		max: null,
		reason: 'invalid-series',
	},
	{
		name: 'an unsupported palette slot',
		parts: [{ label: 'Connect', value: '4', series: '4' }],
		scale: null,
		max: null,
		reason: 'invalid-series',
	},
	{
		name: 'duplicate phase names',
		parts: [
			{ label: 'Connect', value: '4', series: '1' },
			{ label: 'Connect', value: '5', series: '2' },
		],
		scale: null,
		max: null,
		reason: 'duplicate-phase',
	},
	{
		name: 'duplicate palette slots',
		parts: [
			{ label: 'Connect', value: '4', series: '1' },
			{ label: 'Work', value: '5', series: '1' },
		],
		scale: null,
		max: null,
		reason: 'duplicate-phase',
	},
	{
		name: 'more than three phases',
		parts: [
			{ label: 'Connect', value: '1', series: '1' },
			{ label: 'Work', value: '2', series: '2' },
			{ label: 'Transfer', value: '3', series: '3' },
			{ label: 'Wait', value: '4', series: '4' },
		],
		scale: null,
		max: null,
		reason: 'invalid-series',
	},
	{
		name: 'an unsupported scale',
		parts: phases,
		scale: 'percent',
		max: null,
		reason: 'invalid-scale',
	},
	{
		name: 'elapsed scale without a maximum',
		parts: phases,
		scale: 'elapsed',
		max: null,
		reason: 'invalid-scale',
	},
	{
		name: 'elapsed scale with a zero maximum before zero classification',
		parts: phases.map((part) => ({ ...part, value: '0' })),
		scale: 'elapsed',
		max: '0',
		reason: 'invalid-scale',
	},
	{
		name: 'elapsed scale with a negative maximum',
		parts: phases,
		scale: 'elapsed',
		max: '-10',
		reason: 'invalid-scale',
	},
	{
		name: 'elapsed scale with a non-finite maximum',
		parts: phases,
		scale: 'elapsed',
		max: 'Infinity',
		reason: 'invalid-scale',
	},
	{
		name: 'elapsed total above its maximum',
		parts: phases,
		scale: 'elapsed',
		max: '227',
		reason: 'exceeds-scale',
	},
	{
		name: 'decimal total just above its maximum',
		parts: [
			{ label: 'Connect', value: '0.1', series: '1' },
			{ label: 'Work', value: '0.20000000000000004', series: '2' },
		],
		scale: 'elapsed',
		max: '0.3',
		reason: 'exceeds-scale',
	},
	{
		name: 'decimal total clearly above its maximum',
		parts: [
			{ label: 'Connect', value: '0.1', series: '1' },
			{ label: 'Work', value: '0.21', series: '2' },
		],
		scale: 'elapsed',
		max: '0.3',
		reason: 'exceeds-scale',
	},
])('breakdown parser rejects $name', ({ parts, scale, max, reason }): void => {
	expect(parseBreakdownValue(parts, scale, max)).toEqual({ kind: 'invalid', reason });
});
