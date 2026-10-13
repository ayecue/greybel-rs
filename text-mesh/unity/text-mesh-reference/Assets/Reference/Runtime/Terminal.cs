using System.Collections.Generic;
using TMPro;
using UnityEngine;
using UnityEngine.UI;

namespace TextMeshReference
{
    public sealed class Terminal : MonoBehaviour
    {
        private readonly List<TextMeshProUGUI> lines = new List<TextMeshProUGUI>();
        private TMP_FontAsset font;
        private TMP_SpriteAsset sprites;
        private TextSettings settings;
        private FrameSpec frame;
        private RectTransform content;
        private Color32 textColor;

        public RectTransform Content => content;

        public static Terminal Create(
            RectTransform parent,
            TMP_FontAsset font,
            TMP_SpriteAsset sprites,
            Setup setup
        )
        {
            var root = new GameObject(
                "Terminal",
                typeof(RectTransform),
                typeof(CanvasRenderer),
                typeof(Image)
            );
            var rect = (RectTransform)root.transform;
            rect.SetParent(parent, false);
            Stretch(rect);
            var terminal = root.AddComponent<Terminal>();
            terminal.font = font;
            terminal.sprites = sprites;
            terminal.settings = setup.text;
            terminal.frame = setup.frame;
            terminal.textColor = setup.theme.text.ToColor32();
            root.GetComponent<Image>().color = setup.theme.background.ToColor32();
            terminal.content = CreateContent(rect, setup.frame);
            return terminal;
        }

        public void Print(string text)
        {
            foreach (var line in TerminalLineContainer.Split(text))
            {
                AddLine(line);
            }
        }

        public void Clear()
        {
            foreach (var line in lines)
            {
                if (Application.isPlaying)
                {
                    Destroy(line.gameObject);
                }
                else
                {
                    DestroyImmediate(line.gameObject);
                }
            }
            lines.Clear();
        }

        public IReadOnlyList<TextMeshProUGUI> Lines => lines;

        private static RectTransform CreateContent(RectTransform parent, FrameSpec frame)
        {
            var go = new GameObject(
                "Content",
                typeof(RectTransform),
                typeof(VerticalLayoutGroup),
                typeof(ContentSizeFitter)
            );
            var rect = (RectTransform)go.transform;
            rect.SetParent(parent, false);
            rect.anchorMin = new Vector2(0f, 1f);
            rect.anchorMax = new Vector2(1f, 1f);
            rect.pivot = new Vector2(0f, 1f);
            rect.anchoredPosition = Vector2.zero;
            rect.sizeDelta = Vector2.zero;
            var layout = go.GetComponent<VerticalLayoutGroup>();
            var padding = Mathf.RoundToInt(frame.padding);
            layout.padding = new RectOffset(padding, padding, padding, padding);
            layout.spacing = frame.itemGap;
            layout.childAlignment = TextAnchor.UpperLeft;
            layout.childControlWidth = true;
            layout.childControlHeight = true;
            layout.childForceExpandWidth = true;
            layout.childForceExpandHeight = false;
            go.GetComponent<ContentSizeFitter>().verticalFit = ContentSizeFitter
                .FitMode
                .PreferredSize;
            return rect;
        }

        private TextMeshProUGUI AddLine(string line)
        {
            var go = new GameObject(
                "PrototypeTMP",
                typeof(RectTransform),
                typeof(CanvasRenderer),
                typeof(LayoutElement)
            );
            go.transform.SetParent(content, false);
            go.GetComponent<LayoutElement>().minHeight = frame.itemMinHeight;
            var text = go.AddComponent<TextMeshProUGUI>();
            text.font = font;
            text.spriteAsset = sprites;
            text.fontSize = settings.fontSize;
            text.fontWeight = (FontWeight)settings.fontWeight;
            text.fontStyle = FontStyles.Normal;
            text.color = textColor;
            text.horizontalAlignment = (HorizontalAlignmentOptions)settings.HorizontalAlignment;
            text.verticalAlignment = (VerticalAlignmentOptions)settings.VerticalAlignment;
            text.characterSpacing = settings.characterSpacing;
            text.wordSpacing = settings.wordSpacing;
            text.lineSpacing = settings.lineSpacing;
            text.lineSpacingAdjustment = settings.lineSpacingMax;
            text.paragraphSpacing = settings.paragraphSpacing;
            text.enableWordWrapping = settings.enableWordWrapping != 0;
            text.wordWrappingRatios = settings.wordWrappingRatios;
            text.overflowMode = TextOverflowModes.Overflow;
            text.enableKerning = settings.enableKerning != 0;
            text.extraPadding = settings.enableExtraPadding != 0;
            text.richText = settings.isRichText != 0;
            text.parseCtrlCharacters = settings.parseCtrlCharacters != 0;
            text.isOrthographic = settings.isOrthographic != 0;
            text.tintAllSprites = settings.tintAllSprites != 0;
            text.overrideColorTags = settings.overrideHtmlColors != 0;
            text.margin = settings.Margin;
            text.raycastTarget = false;
            text.text = line;
            lines.Add(text);
            return text;
        }

        private static void Stretch(RectTransform rect)
        {
            rect.anchorMin = Vector2.zero;
            rect.anchorMax = Vector2.one;
            rect.pivot = new Vector2(0f, 1f);
            rect.anchoredPosition = Vector2.zero;
            rect.sizeDelta = Vector2.zero;
        }
    }
}
