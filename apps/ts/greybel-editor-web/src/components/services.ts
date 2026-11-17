import { createContext } from 'preact';
import { useContext } from 'preact/hooks';

import type { ManifestHelp } from '../editor/manifest-help.js';
import type { TooltipHighlighter } from '../editor/tooltip/highlighter.js';
import type { Layout } from '../layout.js';
import type { LanguageServerSettings } from '../project/language-server-settings.js';
import type { PathResolver } from '../project/paths.js';
import type { ProjectStorage } from '../project/storage.js';
import type { WasmLoader } from '../wasm.js';

/** The page's shared objects, created once in `main.tsx`. */
export interface ServiceContainer {
  readonly paths: PathResolver;
  readonly storage: ProjectStorage;
  readonly wasm: WasmLoader;
  readonly layout: Layout;
  readonly highlighter: TooltipHighlighter;
  readonly manifestHelp: ManifestHelp;
  readonly languageServerSettings: LanguageServerSettings;
}

export const ServiceContext = createContext<ServiceContainer | undefined>(undefined);

export function useServices(): ServiceContainer {
  const services = useContext(ServiceContext);

  if (services === undefined) throw new Error('A component needs ServiceContext.');

  return services;
}
