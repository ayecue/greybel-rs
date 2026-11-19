import vscode, { ExtensionContext } from 'vscode';

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
import { Configuration } from './settings';
import { GameSetupCommand } from './setup';
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
  const world = new WorldProcess(
    new ProcessLauncher(context, 'greyhack-world-server').getBinaryPath(),
    new ProcessLauncher(context, 'greybel-runtime').getBinaryPath(),
    () => {
      const configuration = new Configuration();

      return {
        port: configuration.get('world.port'),
        idleTimeout: configuration.get('world.idleTimeout'),
        folder: configuration.get('world.folder')
      };
    },
    (reason) => void vscode.window.showWarningMessage(`The debug session uses a world of this window: ${reason}.`)
  );
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
  new GameSetupCommand(context, new ProcessLauncher(context, 'greyhack-setup').getBinaryPath());
}

export function deactivate() {}
