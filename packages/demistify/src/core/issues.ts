interface AuthoringIssue {
	readonly element: Element | undefined;
	readonly message: string;
}

const reportedIssues: AuthoringIssue[] = [];
const reportedKeys = new Set<string>();
let panel: HTMLElement | undefined;
let renderFrame: number | undefined;

function describeElement(element: Element | undefined): string {
	if (!element) return 'page';
	return element.id ? `${element.localName}#${element.id}` : element.localName;
}

function renderPanel(): void {
	renderFrame = undefined;
	if (!panel) {
		panel = document.createElement('aside');
		panel.className = 'ex-issues';
		panel.setAttribute('role', 'status');
		panel.setAttribute('aria-label', 'Authoring issues');
		document.body.append(panel);
	}
	const heading = document.createElement('strong');
	heading.textContent = `${reportedIssues.length} authoring issue${reportedIssues.length === 1 ? '' : 's'}`;
	const list = document.createElement('ol');
	for (const issue of reportedIssues) {
		const item = document.createElement('li');
		const button = document.createElement('button');
		button.type = 'button';
		button.textContent = `${describeElement(issue.element)}: ${issue.message}`;
		button.addEventListener('click', (): void => {
			issue.element?.scrollIntoView({ block: 'center' });
			issue.element?.classList.add('ex-issue-target');
		});
		item.append(button);
		list.append(item);
	}
	panel.replaceChildren(heading, list);
}

/** Records a visible authoring problem once; the page shows a panel and the console a warning. */
export function reportIssue(element: Element | undefined, message: string): void {
	const key = `${describeElement(element)}|${message}`;
	if (reportedKeys.has(key)) return;
	reportedKeys.add(key);
	reportedIssues.push({ element, message });
	element?.classList.add('ex-issue-target');
	// oxlint-disable-next-line no-console -- headless page checks read authoring issues from the console.
	console.warn(`[demistify] ${describeElement(element)}: ${message}`);
	renderFrame ??= requestAnimationFrame(renderPanel);
}

/** Reports duplicate ids and unknown ex-* elements once the vocabulary is registered. */
export function reportStructuralIssues(): void {
	const seenIds = new Set<string>();
	for (const element of document.querySelectorAll('[id]:not(svg *)')) {
		if (seenIds.has(element.id))
			reportIssue(element, `Duplicate id "${element.id}"; ids must be unique.`);
		seenIds.add(element.id);
	}
	for (const element of document.querySelectorAll('*')) {
		if (element.localName.startsWith('ex-') && !customElements.get(element.localName))
			reportIssue(element, `Unknown element <${element.localName}>.`);
	}
}
