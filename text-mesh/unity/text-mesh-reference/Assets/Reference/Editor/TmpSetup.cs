using System.Collections.Generic;
using System.Linq;
using TMPro;
using UnityEditor;
using UnityEngine;

namespace TextMeshReference.Editor
{
    public static class TmpSetup
    {
        private const string SettingsPath = "Assets/TextMesh Pro/Resources/TMP Settings.asset";

        public static void Apply(Setup setup, IReadOnlyDictionary<string, TMP_FontAsset> fonts)
        {
            var settings = AssetDatabase.LoadAssetAtPath<TMP_Settings>(SettingsPath);
            var serializedSettings = new SerializedObject(settings);
            serializedSettings.FindProperty("m_enableWordWrapping").boolValue = true;
            serializedSettings.FindProperty("m_enableKerning").boolValue = true;
            serializedSettings.FindProperty("m_enableExtraPadding").boolValue = false;
            serializedSettings.FindProperty("m_enableTintAllSprites").boolValue = false;
            serializedSettings.FindProperty("m_enableParseEscapeCharacters").boolValue = true;
            serializedSettings.FindProperty("m_missingGlyphCharacter").intValue =
                setup.missingGlyphCharacter;
            serializedSettings.FindProperty("m_defaultFontAssetPath").stringValue =
                "Fonts & Materials/";
            serializedSettings.FindProperty("m_defaultSpriteAssetPath").stringValue =
                "Sprite Assets/";
            serializedSettings.FindProperty("m_defaultSpriteAsset").objectReferenceValue = null;
            serializedSettings.FindProperty("m_defaultFontAsset").objectReferenceValue = fonts[
                setup.terminal
            ];
            var fallbacks = serializedSettings.FindProperty("m_fallbackFontAssets");
            var global = setup.globalFallbacks.Where(fonts.ContainsKey).ToList();
            fallbacks.arraySize = global.Count;
            for (var index = 0; index < global.Count; index++)
            {
                fallbacks.GetArrayElementAtIndex(index).objectReferenceValue = fonts[global[index]];
            }
            serializedSettings.ApplyModifiedPropertiesWithoutUndo();
            AssetDatabase.SaveAssets();
        }

        public static string DescribeStyleSheet()
        {
            var styles = CreateDefaultStyles();
            return styles.Count == 0
                ? "no default style sheet"
                : string.Join(
                    "\n",
                    styles.Select(style =>
                        $"{style.name}: {style.styleOpeningDefinition} … {style.styleClosingDefinition}"
                    )
                );
        }

        public static List<TMP_Style> CreateDefaultStyles()
        {
            var sheet = TMP_Settings.defaultStyleSheet;
            if (sheet == null)
            {
                return new List<TMP_Style>();
            }
            var field = typeof(TMP_StyleSheet).GetField(
                "m_StyleList",
                System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic
            );
            return (List<TMP_Style>)field.GetValue(sheet);
        }
    }
}
