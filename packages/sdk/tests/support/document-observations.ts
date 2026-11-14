// What a run shows of document conventions: outline headings, asking-text tokens and
// target anchor states, read from report values.
import type { TargetAnchor } from '../../src/index.ts';

/** The text of the `<h2>` heading a section's raw text starts with. */
export function headingOf(text: string): string | undefined {
	return /^<h2[^>]*>([^<]*)<\/h2>/u.exec(text)?.[1];
}

/** `[name: value]` tokens of asking text, in order. */
export function tokensOf(text: string): string[] {
	return text.match(/\[[a-z-]+: [^\]]*\]/gu) ?? [];
}

/** A target anchor as [status, reason]; the reason exists only when unsupported. */
export function anchorState(
	anchor: TargetAnchor | null | undefined,
): readonly [string, string] | null {
	if (anchor === null || anchor === undefined) return null;
	return [anchor.status, anchor.status === 'unsupported' ? anchor.reason : ''];
}
