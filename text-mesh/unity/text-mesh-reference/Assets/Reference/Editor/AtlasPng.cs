using UnityEngine;

namespace TextMeshReference.Editor
{
    public static class AtlasPng
    {
        public static byte[] ReadAlphaPng(Texture2D atlas)
        {
            var source = atlas;
            if (!atlas.isReadable)
            {
                var copy = RenderTexture.GetTemporary(
                    atlas.width,
                    atlas.height,
                    0,
                    RenderTextureFormat.ARGB32
                );
                Graphics.Blit(atlas, copy);
                var previous = RenderTexture.active;
                RenderTexture.active = copy;
                source = new Texture2D(atlas.width, atlas.height, TextureFormat.RGBA32, false);
                source.ReadPixels(new Rect(0, 0, atlas.width, atlas.height), 0, 0);
                source.Apply();
                RenderTexture.active = previous;
                RenderTexture.ReleaseTemporary(copy);
            }
            var pixels = source.GetPixels32();
            var gray = new Texture2D(atlas.width, atlas.height, TextureFormat.RGB24, false);
            var values = new Color32[pixels.Length];
            for (var p = 0; p < pixels.Length; p++)
            {
                var alpha = pixels[p].a;
                values[p] = new Color32(alpha, alpha, alpha, 255);
            }
            gray.SetPixels32(values);
            var png = gray.EncodeToPNG();
            UnityEngine.Object.DestroyImmediate(gray);
            if (source != atlas)
            {
                UnityEngine.Object.DestroyImmediate(source);
            }
            return png;
        }
    }
}
