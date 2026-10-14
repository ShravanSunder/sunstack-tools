import { escapeHtml, htmlDocument } from './html-document.ts';
import type { UsageMeasurement } from './usage-measurement.ts';

export interface PrototypeMeasurement extends UsageMeasurement {
	readonly id: string;
	readonly title: string;
	readonly artifactBytes: number;
	readonly artifactGzipBytes: number;
	readonly gains: readonly string[];
	readonly costs: readonly string[];
	readonly href: string;
}
export interface SharedAssetMeasurement {
	readonly name: string;
	readonly bytes: number;
}
export interface ComparisonReportProps {
	readonly measurements: readonly PrototypeMeasurement[];
	readonly sharedAssets: readonly SharedAssetMeasurement[];
	readonly existingInputs: readonly SharedAssetMeasurement[];
}

function assetTable(assets: readonly SharedAssetMeasurement[]): string {
	return `<table><tr><th>Asset</th><th>Bytes</th></tr>${assets.map((asset): string => `<tr><td>${escapeHtml(asset.name)}</td><td>${asset.bytes.toLocaleString()}</td></tr>`).join('')}</table>`;
}

export function comparisonReport(props: ComparisonReportProps): string {
	const usageRows = props.measurements
		.map(
			(item): string =>
				`<tr><td><a href="${item.id}.html">${escapeHtml(item.title)}</a></td><td>${item.usageTokens.toLocaleString()}</td><td>${item.usageBytes.toLocaleString()}</td><td>${item.authorFiles.map((file): string => `<code>${escapeHtml(file.path)}</code><br>`).join('')}</td></tr>`,
		)
		.join('');
	const deliveryRows = props.measurements
		.map(
			(item): string =>
				`<tr><td>${escapeHtml(item.title)}</td><td>${item.artifactBytes.toLocaleString()}</td><td>${item.artifactGzipBytes.toLocaleString()}</td></tr>`,
		)
		.join('');
	const fileLedger = props.measurements
		.map(
			(item): string =>
				`<details><summary>${escapeHtml(item.title)} · exact counted files</summary><table><tr><th>File</th><th>Tokens</th><th>UTF-8 bytes</th><th>SHA-256</th></tr>${item.authorFiles.map((file): string => `<tr><td><code>${escapeHtml(file.path)}</code></td><td>${file.tokens.toLocaleString()}</td><td>${file.bytes.toLocaleString()}</td><td><code>${file.sha256}</code></td></tr>`).join('')}</table></details>`,
		)
		.join('');
	return htmlDocument(
		'Five prototypes · comparison report',
		`
		<h1>One explanation, five presentations</h1>
		<p><a href="../..">Open all five separate pages</a></p>
		<p>Every usage explains the captured planning DAG: three perspectives, 25 nodes, 24 relationships, all jobs/statuses/boundaries/source references, and the same current/change/proposed explanation. The snapshot is dated 2026-10-14.</p>
		<h2>Authoring usage only</h2>
		<p>Counted: complete per-example authored documents, their composition, and any new graph patch. Excluded: library implementations, CSS, fonts, diagram engines, build scripts, generated HTML/SVG, and the existing input corpus. Library creation and possible future library ideas contribute zero to these counts.</p>
		<table><thead><tr><th>Prototype</th><th>Usage tokens</th><th>Usage bytes</th><th>Counted files</th></tr></thead><tbody>${usageRows}</tbody></table>
		<p>Tokens use <code>o200k_base</code> from local <code>js-tiktoken@1.0.21</code>. Each exact UTF-8 source file is encoded separately, including whitespace, and file counts are summed. Literal special-token-looking text is ordinary content. These are deterministic source-token counts, not billed GPT-6.1 tokens, agent-generation cost, repair cost, or comprehension results. <a href="https://github.com/dqbd/tiktoken/tree/main/js">Tokenizer source</a>.</p>
		${fileLedger}
		<h2>Existing inputs, available to all five</h2>
		<p>The original snapshot and its extracted views/base are provided inputs. Referencing an existing diagram is allowed. Redeclaring it in MDX/JSON counts as authored usage; a reference counts only the written reference. Thus the table measures each format's reuse capability as well as its syntax. Newly authored explanatory prose is counted in every variant; the graph prototype counts its full Markdown document plus the patch.</p>
		${assetTable(props.existingInputs)}
		<h2>Delivery sizes — separate from authoring tokens</h2>
		<p>Standalone artifacts embed reusable code or rendered assets. Their bytes describe delivery and local packaging; they never enter the usage-token table.</p>
		<table><tr><th>Prototype</th><th>Standalone bytes</th><th>Gzip bytes</th></tr>${deliveryRows}</table>
		<h2>Reusable HTML Islands library — excluded from usage</h2>
		${assetTable(props.sharedAssets)}
		<p>The existing component library is reused. The additional <a href="html-islands-capabilities-live.html">HTML Islands capability showcase</a> demonstrates layers, timelines, comparisons and file trees separately; it is outside every comparison total.</p>
		<p><a href="html-islands-composition-live.html">Mixed Markdown + Mermaid composition</a> demonstrates the supplied styling study's decision, inset answer, rationale and contributor-note treatments. It also stays outside the benchmark.</p>
		<p><a href="html-islands-layouts-live.html">Grid, flow and absolute whiteboard layouts</a> shows the same islands with arrows and modal/sidebar detail inspection. HTML owns positioning; the library only renders content and connector overlays. D2 has been removed from Prototype 5.</p>
		<h2>What differs in the five presentations</h2>
		${props.measurements.map((item): string => `<h3>${escapeHtml(item.title)}</h3><ul>${[...item.gains, ...item.costs].map((finding): string => `<li>${escapeHtml(finding)}</li>`).join('')}</ul>`).join('')}
		<h2>Proof boundary</h2>
		<p>The complete usages are translated fixtures. Automated parity checks preserve their source facts and change story. Native diagrams/Markdown, upstream MDX and Quick, and the local directive/graph paths render for real. Browser inspection establishes visible results separately. No generation-time, reliability, performance-distribution or human-comprehension winner is claimed.</p>
		<p>Quick uses cards and textual edges; native Mermaid supports beautiful-mermaid's subset; D2 runs during packaging; graph patches describe a flat identity model and independently laid-out views. Direct file navigation is blocked by this host's browser policy. Verification is over local HTTP; WebView and direct file opening remain unverified. No deployment or paid model call is part of this experiment.</p>
	`,
	);
}
