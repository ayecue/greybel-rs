using System;
using UnityEngine;

namespace TextMeshReference
{
    [Serializable]
    public sealed class TextSettings
    {
        public float fontSize;
        public int fontWeight;
        public int HorizontalAlignment;
        public int VerticalAlignment;
        public float characterSpacing;
        public float wordSpacing;
        public float lineSpacing;
        public float lineSpacingMax;
        public float paragraphSpacing;
        public float charWidthMaxAdj;
        public int enableWordWrapping;
        public float wordWrappingRatios;
        public int enableKerning;
        public int enableExtraPadding;
        public int isRichText;
        public int parseCtrlCharacters;
        public int isOrthographic;
        public int tintAllSprites;
        public int overrideHtmlColors;
        public float[] margin;

        public Vector4 Margin => new Vector4(margin[0], margin[1], margin[2], margin[3]);
    }
}
