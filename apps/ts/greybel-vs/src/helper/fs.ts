import { crlf, LF } from 'crlf-normalize';
import vscode, { Uri } from 'vscode';

const fs = vscode.workspace.fs;

export class FileSystemManager {
  async readFile(targetUri: Uri, quiet: boolean = false): Promise<Uint8Array | null> {
    try {
      return await fs.readFile(targetUri);
    } catch (error) {
      if (!quiet) console.error(error);
    }

    return null;
  }

  async readText(targetUri: Uri, quiet: boolean = false): Promise<string | null> {
    const bytes = await this.readFile(targetUri, quiet);

    if (bytes != null) {
      const content = new TextDecoder().decode(bytes);

      return crlf(content, LF);
    }

    return null;
  }

  getWorkspaceFolderUri(source: Uri): Uri | null {
    const folders = vscode.workspace.workspaceFolders;

    return folders.find((folder) => source.path.startsWith(folder.uri.path))?.uri ?? null;
  }

  async getFirstExistingUri(mainUri: Uri, ...altUris: Uri[]): Promise<Uri | null> {
    const primaryBytes = await this.readFile(mainUri, true);

    if (primaryBytes != null) return mainUri;

    if (altUris.length === 0) {
      return null;
    }

    try {
      const existingUri = await Promise.any(
        altUris.map(async (uri) => {
          const bytes = await this.readFile(uri, true);

          if (bytes != null) return uri;
          throw new Error('Alternative path could not resolve');
        })
      );

      if (existingUri != null) {
        return existingUri;
      }

      return null;
    } catch (error) {
      return null;
    }
  }
}

export class CachedFileSystemManager extends FileSystemManager {
  private fileContentCache: Map<string, Uint8Array>;
  private existingUriCache: Map<string, Uri>;

  constructor() {
    super();
    this.fileContentCache = new Map<string, Uint8Array>();
    this.existingUriCache = new Map<string, Uri>();
  }

  async readFile(targetUri: Uri, quiet: boolean = false): Promise<Uint8Array | null> {
    const key = targetUri.toString();
    const result = this.fileContentCache.get(key);

    if (result !== undefined) {
      return result;
    }

    const content = await super.readFile(targetUri, quiet);

    this.fileContentCache.set(key, content);

    return content;
  }

  async getFirstExistingUri(mainUri: Uri, ...altUris: Uri[]): Promise<Uri | null> {
    const key = mainUri.toString();
    const cachedUri = this.existingUriCache.get(key);

    if (cachedUri !== undefined) {
      return cachedUri;
    }

    const result = await super.getFirstExistingUri(mainUri, ...altUris);

    this.existingUriCache.set(key, result);

    return result;
  }
}

export const globalFileSystem = new FileSystemManager();
