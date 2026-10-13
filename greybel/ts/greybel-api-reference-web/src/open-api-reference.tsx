import { MessageTargetDriver, WorkerHost, type MessageTarget } from 'duplex-driver-worker';
import { render } from 'preact';
import { serveWasm, type WasmSource } from 'wasm-host/worker';

import styles from './api-reference.css?inline';
import { App } from './components/app.js';
import { ReferenceTab } from './components/tabs.js';
import { Link } from './protocol.generated.js';
import { ReaderChannel } from './reader.generated.js';
import { OpenDocument, PageChannel } from './workbench-page.generated.js';
import ApiReferenceWorker from './worker.ts?worker&inline';

const STYLE_ID = 'api-reference-styles';

export interface ApiReferenceOptions {
  /** The package's `greybel-api-reference-web.wasm`: its URL, or its bytes or module, or the promise of one. */
  readonly wasm: WasmSource | PromiseLike<WasmSource>;

  readonly editor?: MessageTarget;

  readonly documentationUrl?: string;

  readonly query?: string;

  readonly excludedTags?: readonly string[];

  readonly onExcludedTagsChange?: (tags: readonly string[]) => void;

  /** The id of the part of the guide, type or member to show first, such as `guide-values-lists`. */
  readonly anchor?: string;

  /** The tab shown first: the API's types and members by default. */
  readonly tab?: ReferenceTab;

  readonly onTabChange?: (tab: ReferenceTab) => void;
}

export interface ApiReference {
  readonly ready: Promise<void>;

  close(): void;
}

export function openApiReference(root: HTMLElement, options: ApiReferenceOptions): ApiReference {
  const { editor, documentationUrl } = options;
  const worker = new ApiReferenceWorker();

  serveWasm(worker, options.wasm);

  const host = new WorkerHost(worker, { reader: { channelClass: ReaderChannel } });
  const editorChannel = editor === undefined ? undefined : new PageChannel(new MessageTargetDriver(editor));

  adoptStyles(root.ownerDocument);
  root.classList.add('api-reference');

  const ready = new Promise<void>((resolve, reject) =>
    render(
      <App
        reader={host.channels.reader}
        query={options.query ?? ''}
        excludedTags={new Set(options.excludedTags ?? ['detached'])}
        onExcludedTagsChange={options.onExcludedTagsChange}
        tab={options.tab ?? ReferenceTab.Api}
        onTabChange={options.onTabChange}
        onComplete={resolve}
        onFailure={reject}
        openExample={
          editorChannel === undefined ? undefined : (content) => void editorChannel.send(new OpenDocument({ content }))
        }
        anchor={options.anchor}
        copyLink={documentationUrl === undefined ? undefined : (link) => copyLink(documentationUrl, link)}
      />,
      root
    )
  );

  return {
    ready,
    close: () => {
      render(null, root);
      editorChannel?.close();
      host.close();
    }
  };
}

/**
 * Copies the address of `link` in the documentation at `documentationUrl`: a member as `?filter=owner.name`, a part of
 * the guide as `?tab=guide#anchor`.
 */
function copyLink(documentationUrl: string, link: Link): void {
  const url = new URL(documentationUrl);

  if (link instanceof Link.Member) {
    url.searchParams.set('filter', `${link.owner}.${link.name}`);
  } else if (link instanceof Link.Page) {
    url.searchParams.set('tab', ReferenceTab.Guide);
    url.hash = link.anchor;
  } else {
    return;
  }

  void navigator.clipboard.writeText(url.toString());
}

function adoptStyles(doc: Document): void {
  if (doc.getElementById(STYLE_ID) !== null) return;

  const style = doc.createElement('style');

  style.id = STYLE_ID;
  style.textContent = styles;
  doc.head.append(style);
}
