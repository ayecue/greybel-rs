import { ExtensionContext } from 'vscode';

import { activate as activateApiReference } from './api-reference';
import { activate as activateBuild } from './build';
import { activate as activateNextError } from './next-error';
import { activate as activatePreview } from './preview';
import { activate as activateShare } from './share';
import { activate as activateSnippets } from './snippets';
import { activate as activateTransform } from './transform';
import { activate as activateDebug } from './debug';
import { activate as activateLanguageClient } from './language-client-web';
import { World } from 'greyhack-world-operations';

import { createRuntimeLauncher } from './launcher/runtime';
import { createWorkerLauncher } from './launcher/worker';
import { WorldWorker } from './launcher/world-worker';
import { ProjectReader } from './project';
import { activate as activateProfilePicker } from './project/profile-picker';
import { ProjectSettings } from './project/settings';

export async function activate(context: ExtensionContext) {
  const settings = await ProjectSettings.load(context);
  const projects = new ProjectReader(activateLanguageClient(context), settings);

  settings.start(context);
  activateProfilePicker(context, projects);

  const world = new WorldWorker(context);
  const runtimeLauncher = createRuntimeLauncher(world, (port) =>
    createWorkerLauncher(context, 'runtime-web', port === undefined ? {} : { ports: [{ protocol: World, port }] })()
  );
  const builderLauncher = createWorkerLauncher(context, 'builder-web');

  activateDebug(context, runtimeLauncher, world, projects);
  activateBuild(context, builderLauncher, projects);
  activateTransform(context, builderLauncher, projects);
  activateNextError(context);

  activateApiReference(context);
  activateSnippets(context);
  activateShare(context);
  activatePreview(context);
}

export function deactivate() {}
