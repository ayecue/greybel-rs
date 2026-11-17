import { ExtensionContext } from 'vscode';

import { ApiReference } from './api-reference';
import { BuildCommand } from './build';
import { DebugController } from './debug';
import { ImportCommand } from './import';
import { LanguageServer } from './language-client';
import { ProcessLauncher } from './launcher/process';
import { RuntimePeerLauncher } from './launcher/runtime';
import { WorldProcess } from './launcher/world-process';
import { NextErrorCommand } from './next-error';
import { Preview } from './preview';
import { ProjectReader } from './project';
import { ProfilePicker } from './project/profile-picker';
import { ProjectSettings } from './project/settings';
import { ShareCommand } from './share';
import { SnippetCommand } from './snippets';
import { TransformCommand } from './transform';

export async function activate(context: ExtensionContext) {
  const settings = await ProjectSettings.load(context);
  const languageServer = new LanguageServer(context);
  const projects = new ProjectReader(languageServer.start(), settings);

  settings.start(context);
  new ProfilePicker(context, projects);

  const preview = new Preview(context);
  const world = new WorldProcess(new ProcessLauncher(context, 'greyhack-world-server').getBinaryPath());
  const runtimeLauncher = new RuntimePeerLauncher(world, (address) =>
    new ProcessLauncher(context, 'greybel-runtime', address === undefined ? [] : ['--world', address]).launch()
  );
  const builderLauncher = new ProcessLauncher(context, 'greybel-builder');

  new DebugController(context, runtimeLauncher, world, projects, preview);
  new BuildCommand(context, builderLauncher, projects);
  new TransformCommand(context, builderLauncher, projects);
  new NextErrorCommand(context);

  new ApiReference(context);
  new SnippetCommand(context);
  new ShareCommand(context, projects);
  new ImportCommand(context, builderLauncher, projects);
}

export function deactivate() {}
