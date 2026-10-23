/** A count compared with one named subject's authored total. */
export type ProgressValue =
	| { readonly kind: 'known'; readonly value: number; readonly max: number }
	| { readonly kind: 'unknown' }
	| { readonly kind: 'empty'; readonly value: 0; readonly max: 0 }
	| {
			readonly kind: 'invalid';
			readonly reason: 'missing-total' | 'not-integer' | 'negative' | 'exceeds-total';
	  };

export interface BreakdownPartInput {
	readonly label: string;
	readonly value: string | null;
	readonly series: string | null;
}

export interface PhaseValue {
	readonly label: string;
	readonly value: number;
	readonly series: 1 | 2 | 3;
}

/** A closed result for one observation and its stable, authored phase identities. */
export type BreakdownValue =
	| {
			readonly kind: 'known';
			readonly parts: readonly PhaseValue[];
			readonly total: number;
			readonly max: number;
			readonly scale: 'share' | 'elapsed';
	  }
	| {
			readonly kind: 'zero';
			readonly parts: readonly PhaseValue[];
			readonly total: 0;
			readonly max: number;
			readonly scale: 'share' | 'elapsed';
	  }
	| {
			readonly kind: 'invalid';
			readonly reason:
				| 'missing-parts'
				| 'invalid-value'
				| 'invalid-series'
				| 'duplicate-phase'
				| 'invalid-scale'
				| 'exceeds-scale';
	  };

function readFiniteNumber(raw: string | null): number | undefined {
	if (raw === null) return undefined;
	const text = raw.trim();
	if (!/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?$/i.test(text)) return undefined;
	const value = Number(text);
	return Number.isFinite(value) ? value : undefined;
}

interface CanonicalNumberDecimal {
	readonly significand: bigint;
	readonly decimalExponent: number;
}

function canonicalDecimalFromNumber(value: number): CanonicalNumberDecimal {
	if (!Number.isFinite(value) || value < 0)
		throw new Error('A canonical decimal requires a finite non-negative number.');

	const groups = /^(?<whole>\d+)(?:\.(?<fraction>\d+))?(?:e(?<exponent>[+-]?\d+))?$/i.exec(
		value.toString(),
	)?.groups;
	const whole = groups?.['whole'];
	if (whole === undefined) throw new Error('A finite number had no canonical decimal form.');

	const fraction = groups?.['fraction'] ?? '';
	return {
		significand: BigInt(`${whole}${fraction}`),
		decimalExponent: Number(groups?.['exponent'] ?? '0') - fraction.length,
	};
}

function sumCanonicalDecimals(values: readonly number[]): CanonicalNumberDecimal {
	const decimals = values.map(canonicalDecimalFromNumber);
	const decimalExponent = Math.min(...decimals.map((decimal): number => decimal.decimalExponent));
	const significand = decimals.reduce(
		(sum, decimal): bigint =>
			sum + decimal.significand * 10n ** BigInt(decimal.decimalExponent - decimalExponent),
		0n,
	);
	return { significand, decimalExponent };
}

function compareCanonicalDecimals(
	left: CanonicalNumberDecimal,
	right: CanonicalNumberDecimal,
): -1 | 0 | 1 {
	const decimalExponent = Math.min(left.decimalExponent, right.decimalExponent);
	const leftSignificand = left.significand * 10n ** BigInt(left.decimalExponent - decimalExponent);
	const rightSignificand =
		right.significand * 10n ** BigInt(right.decimalExponent - decimalExponent);
	return leftSignificand < rightSignificand ? -1 : leftSignificand > rightSignificand ? 1 : 0;
}

function canonicalDecimalToNumber(decimal: CanonicalNumberDecimal): number {
	return Number(`${decimal.significand}e${decimal.decimalExponent}`);
}

/** Parses authored strings once, leaving absence distinct from an authored zero. */
export function parseProgressValue(value: string | null, max: string | null): ProgressValue {
	if (value === null) return { kind: 'unknown' };
	if (max === null) return { kind: 'invalid', reason: 'missing-total' };

	const parsedValue = readFiniteNumber(value);
	const parsedMax = readFiniteNumber(max);
	if (parsedValue === undefined || parsedMax === undefined)
		return { kind: 'invalid', reason: 'not-integer' };
	if (parsedValue < 0 || parsedMax < 0) return { kind: 'invalid', reason: 'negative' };
	if (!Number.isInteger(parsedValue) || !Number.isInteger(parsedMax))
		return { kind: 'invalid', reason: 'not-integer' };
	if (parsedValue > parsedMax) return { kind: 'invalid', reason: 'exceeds-total' };
	if (parsedMax === 0) return { kind: 'empty', value: 0, max: 0 };
	return { kind: 'known', value: parsedValue, max: parsedMax };
}

/**
 * Validates phase identity and values before deriving a share or elapsed layout. Elapsed max is
 * checked before a zero total, so a zero observation cannot hide a missing or invalid denominator.
 */
export function parseBreakdownValue(
	inputs: readonly BreakdownPartInput[],
	scaleInput: string | null,
	maxInput: string | null,
): BreakdownValue {
	const scale = scaleInput ?? 'share';
	if (scale !== 'share' && scale !== 'elapsed') return { kind: 'invalid', reason: 'invalid-scale' };

	let elapsedMax: number | undefined;
	if (scale === 'elapsed') {
		elapsedMax = readFiniteNumber(maxInput);
		if (elapsedMax === undefined || elapsedMax <= 0)
			return { kind: 'invalid', reason: 'invalid-scale' };
	}

	if (inputs.length === 0) return { kind: 'invalid', reason: 'missing-parts' };
	if (inputs.length > 3) return { kind: 'invalid', reason: 'invalid-series' };

	const labels = new Set<string>();
	const seriesValues = new Set<number>();
	const parts: PhaseValue[] = [];
	for (const input of inputs) {
		const label = input.label.trim();
		if (!label) return { kind: 'invalid', reason: 'missing-parts' };

		const value = readFiniteNumber(input.value);
		if (value === undefined || value < 0) return { kind: 'invalid', reason: 'invalid-value' };

		if (input.series !== '1' && input.series !== '2' && input.series !== '3')
			return { kind: 'invalid', reason: 'invalid-series' };
		const series: 1 | 2 | 3 = input.series === '1' ? 1 : input.series === '2' ? 2 : 3;

		if (labels.has(label) || seriesValues.has(series))
			return { kind: 'invalid', reason: 'duplicate-phase' };
		labels.add(label);
		seriesValues.add(series);
		parts.push({ label, value, series });
	}

	const totalDecimal = sumCanonicalDecimals(parts.map((part): number => part.value));
	const total = canonicalDecimalToNumber(totalDecimal);
	if (!Number.isFinite(total)) return { kind: 'invalid', reason: 'invalid-value' };
	if (
		scale === 'elapsed' &&
		elapsedMax !== undefined &&
		compareCanonicalDecimals(totalDecimal, canonicalDecimalFromNumber(elapsedMax)) > 0
	)
		return { kind: 'invalid', reason: 'exceeds-scale' };

	const max = scale === 'share' ? total : (elapsedMax ?? 0);
	if (total === 0) return { kind: 'zero', parts, total: 0, max, scale };
	return { kind: 'known', parts, total, max, scale };
}
