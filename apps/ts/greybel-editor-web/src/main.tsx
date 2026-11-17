import { render } from 'preact';

import { App } from './components/app.js';
import { ServiceContext, type ServiceContainer } from './components/services.js';
import { ManifestHelp } from './editor/manifest-help.js';
import { TooltipHighlighter } from './editor/tooltip/highlighter.js';
import schemaText from './greybel.schema.json?raw';
import { Layout } from './layout.js';
import type { JsonObject } from './project/json.js';
import { LanguageServerSettings } from './project/language-server-settings.js';
import { PathResolver } from './project/paths.js';
import { ProjectStorage } from './project/storage.js';
import { WatchList } from './runtime/watch-list.js';
import { WasmLoader } from './wasm.js';
import './editor.css';

const root = document.getElementById('editor-app');

if (root === null) throw new Error('The page has no #editor-app element.');

const schema = JSON.parse(schemaText) as JsonObject;
const paths = new PathResolver();
const services: ServiceContainer = {
  paths,
  storage: new ProjectStorage(location, localStorage, import.meta.env.EDITOR_SERVICE_URL || undefined, paths),
  wasm: new WasmLoader(),
  layout: new Layout(document.documentElement, localStorage),
  highlighter: new TooltipHighlighter(),
  manifestHelp: new ManifestHelp(schema),
  languageServerSettings: new LanguageServerSettings(schema),
  watches: new WatchList(localStorage)
};

services.layout.restore();
render(
  <ServiceContext.Provider value={services}>
    <App content={await services.storage.loadInitial()} />
  </ServiceContext.Provider>,
  root
);
