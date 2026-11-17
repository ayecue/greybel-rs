import vscode, { ExtensionContext, Uri } from 'vscode';

import { JsonRequest } from './helper/request';
import type { ProjectReader } from './project';

const MANIFEST_NAME = 'greybel.json';
const OPEN_LINK = 'Open link';

/** A workspace of greybel-editor-service: its entry and files by relative path. */
interface SharedWorkspace {
  readonly entry: string;
  readonly files: Record<string, string>;
}

/** Shares a file with every file it imports, and the project's `greybel.json`, as a web editor link. */
export class ShareCommand {
  readonly #projects: ProjectReader;
  readonly #request = new JsonRequest();

  constructor(context: ExtensionContext, projects: ProjectReader) {
    this.#projects = projects;
    context.subscriptions.push(vscode.commands.registerCommand('greybel.share', (uri?: Uri) => this.#share(uri)));
  }

  async #share(documentUri: Uri | undefined = vscode.window.activeTextEditor?.document?.uri): Promise<void> {
    if (documentUri === undefined) return;

    try {
      const workspace = await this.#createWorkspace(documentUri);
      const response = await this.#request.post<{ id: string }>(
        `${process.env.EDITOR_SERVICE_URL}/workspaces`,
        workspace
      );
      const link = `${process.env.GREYBEL_EDITOR_URL}?id=${response.id}`;
      const count = Object.keys(workspace.files).length;

      await vscode.env.clipboard.writeText(link);

      const choice = await vscode.window.showInformationMessage(
        `Editor link for ${count} ${count === 1 ? 'file' : 'files'} created and copied to the clipboard.`,
        { modal: false },
        OPEN_LINK
      );

      if (choice === OPEN_LINK) await vscode.env.openExternal(Uri.parse(link));
    } catch (error: unknown) {
      vscode.window.showErrorMessage(
        `Couldn't share the file: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  /** The document, its imports and `greybel.json`, with paths relative to the project folder. */
  async #createWorkspace(document: Uri): Promise<SharedWorkspace> {
    const project = await this.#projects.read(document);
    const scripts = [document, ...(await this.#projects.getImportedFiles(document))];
    const manifest = project.root === undefined ? undefined : Uri.joinPath(project.root, MANIFEST_NAME);
    const base = this.#findBase(
      [...scripts, ...(manifest === undefined ? [] : [manifest])],
      project.getFolder(document)
    );
    const files: Record<string, string> = {};

    for (const script of scripts) {
      files[this.#toRelativePath(base, script)] = (await vscode.workspace.openTextDocument(script)).getText();
    }

    if (manifest !== undefined) {
      files[this.#toRelativePath(base, manifest)] = new TextDecoder().decode(
        await vscode.workspace.fs.readFile(manifest)
      );
    }

    return { entry: this.#toRelativePath(base, document), files };
  }

  /** `folder` when it contains every file, otherwise the deepest folder containing them all. */
  #findBase(uris: readonly Uri[], folder: Uri): Uri {
    if (uris.every((uri) => uri.path.startsWith(`${folder.path}/`))) return folder;

    const folders = uris.map((uri) => uri.path.split('/').slice(0, -1));
    const common = folders.reduce((shared, segments) => {
      let length = 0;

      while (length < shared.length && shared[length] === segments[length]) length++;

      return shared.slice(0, length);
    });

    return uris[0]!.with({ path: common.join('/') || '/' });
  }

  #toRelativePath(base: Uri, uri: Uri): string {
    const prefix = base.path.endsWith('/') ? base.path : `${base.path}/`;

    return uri.path.startsWith(prefix) ? uri.path.slice(prefix.length) : uri.path.replace(/^\/+/, '');
  }
}
