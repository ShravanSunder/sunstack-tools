import { Worker } from 'node:worker_threads';

import { D2 } from '@terrastruct/d2';

export async function renderD2Sources(
	sources: Readonly<Record<string, string>>,
): Promise<Record<string, string>> {
	const diagramEngine = new D2();
	try {
		// One D2 worker stores one pending resolve/reject pair: requests must be serialized.
		return await Object.entries(sources).reduce(
			async (previousResult, [sourceName, sourceText]): Promise<Record<string, string>> => {
				const renderedSources = await previousResult;
				const compiledDiagram = await diagramEngine.compile(sourceText);
				renderedSources[sourceName] = await diagramEngine.render(compiledDiagram.diagram, {
					...compiledDiagram.renderOptions,
					noXMLTag: true,
					pad: 24,
					center: true,
					salt: sourceName,
				});
				return renderedSources;
			},
			Promise.resolve<Record<string, string>>({}),
		);
	} finally {
		// The installed D2 implementation exposes a Node Worker; its declarations omit lifecycle APIs.
		const workerHandle: unknown = Reflect.get(diagramEngine, 'worker');
		if (workerHandle instanceof Worker) await workerHandle.terminate();
	}
}
