import themeJSON from './greyscript.theme.json' with { type: 'json' };
import signatureTmLanguageJSON from './greyscript-signature.tmLanguage.json' with { type: 'json' };
import tmLanguageJSON from './greyscript.tmLanguage.json' with { type: 'json' };
import languageConfigJSON from './greyscriptLanguageConfig.json' with { type: 'json' };

export const theme: Record<string, unknown> = themeJSON;
export const tmLanguage: Record<string, unknown> = tmLanguageJSON;
export const signatureTmLanguage: Record<string, unknown> = signatureTmLanguageJSON;
export const languageConfig: Record<string, unknown> = languageConfigJSON;
