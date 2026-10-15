import type { ExplanationFixture } from './explanation-fixture.ts';
import type { GraphChangeSet } from './graph-changes.ts';
import { escapeHtml, htmlDocument } from './html-document.ts';

export function graphDocument(
	fixture: ExplanationFixture,
	changeSet: GraphChangeSet,
	diagrams: Readonly<Record<string, string>>,
): string {
	const graphPanels = ['current', 'diff', 'proposed']
		.map(
			(viewName): string =>
				`<section class="graph-panel" id="${viewName}" ${viewName !== 'diff' ? 'hidden' : ''}><h2>${viewName === 'diff' ? 'Changes, including removals' : viewName === 'current' ? 'Current architecture' : 'Proposed architecture'}</h2><figure>${diagrams[viewName]}</figure></section>`,
		)
		.join('');
	const controls = ['current', 'diff', 'proposed']
		.map(
			(viewName): string =>
				`<button type="button" data-view="${viewName}" aria-controls="${viewName}" aria-pressed="${viewName === 'diff'}">${viewName === 'current' ? 'Current' : viewName === 'diff' ? 'Changes' : 'Proposed'}</button>`,
		)
		.join('');
	const changesHtml = changeSet.changes
		.map((change): string => {
			const description =
				change.kind === 'add-node'
					? `Add node: ${change.node.label}`
					: change.kind === 'add-edge'
						? `Add edge: ${change.edge.from} → ${change.edge.to} (${change.edge.label})`
						: change.kind === 'change-node'
							? `Change node: ${change.id} → ${change.label}`
							: `${change.kind === 'remove-node' ? 'Remove node' : 'Remove edge'}: ${change.id}`;
			return `<li>${escapeHtml(description)}</li>`;
		})
		.join('');
	const alternativesHtml = fixture.alternatives
		.map(
			(alternative): string =>
				`<tr><td>${escapeHtml(alternative.name)}</td><td>${escapeHtml(alternative.gain)}</td><td>${escapeHtml(alternative.cost)}</td></tr>`,
		)
		.join('');
	const phasesHtml = fixture.phases
		.map(
			(phase): string =>
				`<li><strong>${escapeHtml(phase.title)}</strong><p>${escapeHtml(phase.body)}</p></li>`,
		)
		.join('');
	return htmlDocument(
		fixture.title,
		`<h1>${escapeHtml(fixture.title)}</h1><p>${escapeHtml(fixture.subtitle)}</p><p>${escapeHtml(fixture.summary)}</p><div class="graph-controls" aria-label="Architecture view">${controls}</div>${graphPanels}<p>Green + “added”, amber + “changed”, red + “removed”. Labels carry status without relying on color.</p><details><summary>Exact semantic changes</summary><ul class="change-list">${changesHtml}</ul></details><aside><strong>Decision</strong><p>${escapeHtml(fixture.decision)}</p></aside><h2>Alternatives</h2><table><thead><tr><th>Alternative</th><th>Gain</th><th>Cost</th></tr></thead><tbody>${alternativesHtml}</tbody></table><h2>Implementation phases</h2><ol>${phasesHtml}</ol><aside class="risk"><strong>Risk</strong><p>${escapeHtml(fixture.risk)}</p></aside><script>
  for (const button of document.querySelectorAll('[data-view]')) {
    button.addEventListener('click', () => {
      for (const panel of document.querySelectorAll('.graph-panel')) panel.hidden = panel.id !== button.dataset.view;
      for (const control of document.querySelectorAll('[data-view]')) control.setAttribute('aria-pressed', String(control === button));
    });
  }
  </script>`,
	);
}
