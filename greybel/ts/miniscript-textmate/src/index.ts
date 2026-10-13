import themeJSON from './miniscript.theme.json' with { type: 'json' };
import tmLanguageJSON from './miniscript.tmLanguage.json' with { type: 'json' };
import languageConfigJSON from './miniscriptLanguageConfig.json' with { type: 'json' };

export const theme: Record<string, unknown> = themeJSON;
export const tmLanguage: Record<string, unknown> = tmLanguageJSON;
export const languageConfig: Record<string, unknown> = languageConfigJSON;
