import { EditorRouter, type FindFiles, type ReadFile } from 'greybel-documents-host-provider';
import { Uri, workspace } from 'vscode';

import { globalFileSystem } from './helper/fs';

export class WorkspaceDocumentProvider extends EditorRouter {
  override async onReadFile(message: ReadFile): Promise<string | null> {
    return globalFileSystem.readText(Uri.parse(message.uri), true);
  }

  override async onFindFiles(message: FindFiles): Promise<string[]> {
    const uris = await workspace.findFiles(message.include, message.exclude ?? undefined);

    return uris.map((uri) => uri.toString());
  }
}
