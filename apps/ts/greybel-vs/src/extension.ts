import { ExtensionContext } from 'vscode';

import { activate as activateApiReference } from './api-reference';
import { activate as activateBuild } from './build';
import { activate as activateDebug } from './debug';
import { activate as activateNextError } from './next-error';
import { activate as activateShare } from './share';
import { activate as activateSnippets } from './snippets';
import { activate as activateTransform } from './transform';
import { activate as activateImport } from './import';
import { activate as activatePreview } from './preview';
import { activate as activateLanguageClient } from './language-client';
import { binaryPath, createProcessLauncher } from './launcher/process';
import { createRuntimeLauncher } from './launcher/runtime';
import { WorldProcess } from './launcher/world-process';
import { ProjectReader } from './project';
import { activate as activateProfilePicker } from './project/profile-picker';
import { ProjectSettings } from './project/settings';

export async function activate(context: ExtensionContext) {
  const settings = await ProjectSettings.load(context);
  const projects = new ProjectReader(activateLanguageClient(context), settings);

  settings.start(context);
  activateProfilePicker(context, projects);

  const world = new WorldProcess(binaryPath(context, 'greyhack-world-server'));
  const runtimeLauncher = createRuntimeLauncher(world, (address) =>
    createProcessLauncher(context, 'greybel-runtime', address === undefined ? [] : ['--world', address])()
  );
  const builderLauncher = createProcessLauncher(context, 'greybel-builder');

  activateDebug(context, runtimeLauncher, world, projects);
  activateBuild(context, builderLauncher, projects);
  activateTransform(context, builderLauncher, projects);
  activateNextError(context);

  activateApiReference(context);
  activateSnippets(context);
  activateShare(context);
  activateImport(context, builderLauncher, projects);
  activatePreview(context);
}

export function deactivate() {}
