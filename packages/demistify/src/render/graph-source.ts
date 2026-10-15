/**
 * The diagram engine reads a flowchart or state edge only when the arrow stands apart from its
 * nodes (`a --> b`, `b -->|yes| c`). `a-->b` silently becomes nodes named `a--` and no edge, and
 * `c -.-> |no| d` drops the edge, so edges are respaced before rendering.
 */
const graphHeader = /^\s*(?:flowchart|graph|stateDiagram)\b/i;
const statementLine =
	/^\s*(?:%%|style\b|classDef\b|class\b|linkStyle\b|click\b|subgraph\b|end\b|direction\b|state\b)/i;
const arrowToken = /^<?(?:-{2,}|={2,}|-\.+-)(?:>|[ox](?=[\s|]|$))?/;
const edgeLabel = /^\s*(\|[^|]*\|)/;

function respaceLine(line: string): string {
	let output = '';
	let depth = 0;
	let quoted = false;
	let index = 0;
	while (index < line.length) {
		const character = line.charAt(index);
		const arrow = quoted || depth > 0 ? null : arrowToken.exec(line.slice(index));
		if (!arrow) {
			if (character === '"') quoted = !quoted;
			else if (!quoted && '[({'.includes(character)) depth += 1;
			else if (!quoted && ')]}'.includes(character)) depth = Math.max(0, depth - 1);
			output += character;
			index += 1;
			continue;
		}
		let token = arrow[0];
		index += token.length;
		const label = edgeLabel.exec(line.slice(index));
		if (label) {
			token += label[1] ?? '';
			index += label[0].length;
		}
		while (line.charAt(index) === ' ' || line.charAt(index) === '\t') index += 1;
		output = `${output.trimEnd()} ${token} `;
	}
	return output.trimEnd();
}

export function respaceGraphEdges(sourceText: string): string {
	if (!graphHeader.test(sourceText)) return sourceText;
	return sourceText
		.split('\n')
		.map((line, index): string =>
			index === 0 || statementLine.test(line) || !/[-=]{2}|-\./.test(line)
				? line
				: respaceLine(line),
		)
		.join('\n');
}

export function hasGraphEdges(sourceText: string): boolean {
	return graphHeader.test(sourceText) && /[-=]{2}|-\.-/.test(sourceText.replace(/^[^\n]*/, ''));
}
