/** Bare text written between islands is Markdown; it becomes a Markdown island in place. */
export function adoptBareMarkdown(container: Element): void {
	let run: Text[] = [];
	const flush = (): void => {
		const first = run[0];
		if (first && run.some((node): boolean => node.data.trim().length > 0)) {
			const script = document.createElement('script');
			script.type = 'text/markdown';
			script.textContent = run.map((node): string => node.data).join('');
			const island = document.createElement('ex-markdown');
			island.append(script);
			first.before(island);
			for (const node of run) node.remove();
		}
		run = [];
	};
	for (const node of Array.from(container.childNodes)) {
		if (node instanceof Text) run.push(node);
		else flush();
	}
	flush();
}
