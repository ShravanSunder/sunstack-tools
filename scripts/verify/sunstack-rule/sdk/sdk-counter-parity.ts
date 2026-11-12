/** Exact CLI decimal-token versus SDK bigint comparison, scoped to file-count DTOs. */
import { requireSdk } from './sdk-target.ts';

const kindCounters = new Set(['matches', 'pieces', 'picked']);
const selectionCounters = new Set([
	'candidates',
	'asked',
	'selected',
	'not_selected',
	'uncertain',
	'incomplete',
]);
const maximumCounter = 18446744073709551615n;

function isRecord(value: unknown): value is Record<string, unknown> {
	return value !== null && typeof value === 'object' && !Array.isArray(value);
}
function isCounterPath(parts: readonly string[]): boolean {
	return (
		parts[0] === 'file_counts' &&
		parts[2] === 'kinds' &&
		((parts.length === 5 && kindCounters.has(parts[4] ?? '')) ||
			(parts.length === 6 && parts[4] === 'selection' && selectionCounters.has(parts[5] ?? '')))
	);
}

export function requireSdkFileCountParity(rawCliReport: string, sdkFileCounts: unknown): void {
	// Reviver source text retains the original token even when JSON's Number value rounded.
	// Opaque objects are private parse receipts, not values accepted from the report itself.
	const integerTokens = new WeakMap<object, string>();
	const parsed: unknown = JSON.parse(
		rawCliReport,
		(_key: string, value: unknown, context?: unknown): unknown => {
			if (
				typeof value === 'number' &&
				isRecord(context) &&
				typeof context['source'] === 'string' &&
				/^-?\d+$/.test(context['source'])
			) {
				const token = Object.freeze({});
				integerTokens.set(token, context['source']);
				return token;
			}
			return value;
		},
	);
	requireSdk(
		isRecord(parsed) && Object.hasOwn(parsed, 'file_counts'),
		'CLI report is missing file_counts',
	);
	requireSdk(
		Array.isArray(parsed['file_counts']) && Array.isArray(sdkFileCounts),
		'file_counts must remain arrays',
	);
	const compare = (cli: unknown, sdk: unknown, parts: readonly string[]): void => {
		const where = parts.join('.');
		const token = isRecord(cli) ? integerTokens.get(cli) : undefined;
		if (isCounterPath(parts)) {
			requireSdk(
				token !== undefined && /^(?:0|[1-9]\d*)$/.test(token) && typeof sdk === 'bigint',
				`Invalid exact counter representation at ${where}`,
			);
			const exact = BigInt(token);
			requireSdk(
				exact <= maximumCounter && sdk >= 0n && sdk <= maximumCounter && exact === sdk,
				`CLI/SDK counter differs at ${where}`,
			);
			return;
		}
		if (token !== undefined) {
			// Non-counter fields keep their ordinary Number type; no arbitrary bigint coercion.
			requireSdk(
				typeof sdk === 'number' && Number.isSafeInteger(sdk) && String(sdk) === token,
				`CLI/SDK unrelated integer differs at ${where}`,
			);
			return;
		}
		if (Array.isArray(cli)) {
			requireSdk(
				Array.isArray(sdk) && cli.length === sdk.length,
				`CLI/SDK array differs at ${where}`,
			);
			cli.forEach((value: unknown, index) => compare(value, sdk[index], [...parts, String(index)]));
			return;
		}
		if (isRecord(cli)) {
			requireSdk(isRecord(sdk), `CLI/SDK object differs at ${where}`);
			const keys = Object.keys(cli);
			const sdkKeys = Object.keys(sdk);
			requireSdk(
				keys.length === sdkKeys.length && keys.every((key, index) => key === sdkKeys[index]),
				`CLI/SDK keys or presence differ at ${where}`,
			);
			for (const key of keys) compare(cli[key], sdk[key], [...parts, key]);
			return;
		}
		requireSdk(Object.is(cli, sdk), `CLI/SDK value differs at ${where}`);
	};
	compare(parsed['file_counts'], sdkFileCounts, ['file_counts']);
}
