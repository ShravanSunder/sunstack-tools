declare module 'subset-font' {
	export default function subsetFont(
		font: Buffer,
		text: string,
		options?: { readonly targetFormat?: 'woff2' | 'woff' | 'sfnt' },
	): Promise<Buffer>;
}
