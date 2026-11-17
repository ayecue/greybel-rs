import { World } from 'greyhack-world-operations';
import { ExtensionContext } from 'vscode';

import { ApiReference } from './api-reference';
import { BuildCommand } from './build';
import { DebugController } from './debug';
import { LanguageServer } from './language-client-web';
import { RuntimePeerLauncher } from './launcher/runtime';
import { WorkerLauncher } from './launcher/worker';
import { WorldWorker } from './launcher/world-worker';
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
  const world = new WorldWorker(context);
  const runtimeLauncher = new RuntimePeerLauncher(world, (port) =>
    new WorkerLauncher(
      context,
      'runtime-web',
      port === undefined ? {} : { ports: [{ protocol: World, port }] }
    ).launch()
  );
  const builderLauncher = new WorkerLauncher(context, 'builder-web');

  new DebugController(context, runtimeLauncher, world, projects, preview);
  new BuildCommand(context, builderLauncher, projects);
  new TransformCommand(context, builderLauncher, projects);
  new NextErrorCommand(context);

  new ApiReference(context);
  new SnippetCommand(context);
  new ShareCommand(context, projects);
}

export function deactivate() {}
