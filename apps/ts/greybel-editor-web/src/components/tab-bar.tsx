import type { ProjectEditor } from '../editor/view.js';

export interface TabBarConfiguration {
  readonly editor: ProjectEditor;
}

export function TabBar({ editor }: TabBarConfiguration) {
  return (
    <div class="tab-bar" role="tablist" aria-label="Open files">
      {editor.tabs.map((path) => (
        <div
          key={path}
          role="tab"
          class={path === editor.path ? 'tab active' : 'tab'}
          aria-selected={path === editor.path}
          data-path={path}
          title={path}
          onClick={() => editor.open(path)}
          onAuxClick={(event) => {
            if (event.button !== 1) return;
            event.preventDefault();
            editor.close(path);
          }}
        >
          <span class="tab-name">{path.split('/').at(-1)}</span>
          <button
            type="button"
            class="close-tab"
            title={`Close ${path}`}
            onClick={(event) => {
              event.stopPropagation();
              editor.close(path);
            }}
          >
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
