import { escapeHtml } from '../../../core/html-text.ts';

export interface ShellRuntime {
	readonly stylesheet: string;
	readonly script: string;
}

const defaultRuntime: ShellRuntime = {
	stylesheet: '<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css">',
	script:
		'<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script>',
};

/** Reuse the runtime binding the initiative's pages already load. */
export function shellRuntime(pages: readonly string[]): ShellRuntime {
	for (const html of pages) {
		const stylesheet = /<link\b[^>]*\btheme\.css\b[^>]*>/iu.exec(html)?.[0];
		const script = /<script\b[^>]*\bruntime\.js\b[^>]*><\/script>/iu.exec(html)?.[0];
		if (stylesheet && script) return { stylesheet, script };
	}
	return defaultRuntime;
}

interface ShellPageInput {
	readonly title: string;
	readonly achieves: string | undefined;
	readonly runtime: ShellRuntime;
	readonly backLink: string | undefined;
}

/**
 * A new project's page: navigation, the record sections a decision or assumption can
 * land in, and delivery progress. Moved passages arrive before the progress table.
 */
export function projectShellPage({ title, achieves, runtime, backLink }: ShellPageInput): string {
	const text = escapeHtml(title);
	return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>${text}</title>
  ${runtime.stylesheet}
  ${runtime.script}
</head>
<body>
  <ex-nav for="content" src="./navigation.generated.json"></ex-nav>
  <ex-document id="content">
${backLink ? `    <p><a href="${escapeHtml(backLink)}">Back to the initiative</a></p>\n` : ''}    <h1>${text}</h1>
${achieves ? `    <p>${escapeHtml(achieves)}</p>\n` : ''}    <ex-section id="open" title="For you to decide" holds="needs-owner"></ex-section>
    <ex-section id="agent" title="Decided without you" holds="decided-by-agent"></ex-section>
    <ex-section id="settled" title="You decided" holds="settled"></ex-section>
    <ex-section id="assumptions" title="Assumed" holds="assumed"></ex-section>
    <ex-deliverables src="./deliverables.generated.json" label="Delivery progress"></ex-deliverables>
  </ex-document>
</body>
</html>
`;
}
