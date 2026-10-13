using System;
using System.Collections.Generic;
using System.IO;
using TMPro;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.UI;

namespace TextMeshReference.Editor
{
    public static class Capture
    {
        private const int DefaultWidth = 1000;
        private const int MaxHeight = 4096;

        [Serializable]
        private sealed class CaptureEntry
        {
            public int line;
            public string file;
            public int width;
            public int height;
            public List<ItemLayout> items = new List<ItemLayout>();
        }

        [Serializable]
        private sealed class ItemLayout
        {
            public string text;
            public float lossyScale;
            public float top;
            public float width;
            public float height;
            public List<CharacterLayout> characters = new List<CharacterLayout>();
            public List<LineLayout> lines = new List<LineLayout>();
            public List<MeshLayout> meshes = new List<MeshLayout>();
        }

        [Serializable]
        private sealed class MeshLayout
        {
            public string material;
            public string texture;
            public List<float> positions = new List<float>();
            public List<float> uvs = new List<float>();
            public List<float> scales = new List<float>();
            public List<int> colors = new List<int>();
        }

        [Serializable]
        private sealed class AssetLayout
        {
            public string name;
            public string material;
            public int atlasWidth;
            public int atlasHeight;
            public int padding;
            public List<string> atlases = new List<string>();
            public List<GlyphLayout> glyphs = new List<GlyphLayout>();
        }

        [Serializable]
        private sealed class GlyphLayout
        {
            public uint unicode;
            public int x;
            public int y;
            public int width;
            public int height;
            public int atlas;
        }

        [Serializable]
        private sealed class CharacterLayout
        {
            public int unicode;
            public int index;
            public bool sprite;
            public bool visible;
            public int line;
            public float origin;
            public float advance;
            public float baseline;
            public float ascender;
            public float descender;
            public float left;
            public float top;
            public float right;
            public float bottom;
            public float pointSize;
            public float scale;
        }

        [Serializable]
        private sealed class LineLayout
        {
            public int first;
            public int last;
            public float ascender;
            public float descender;
            public float baseline;
            public float lineHeight;
            public float width;
            public float length;
            public float marginLeft;
            public float marginRight;
            public int alignment;
        }

        [Serializable]
        private sealed class CaptureIndex
        {
            public int width;
            public List<CaptureEntry> captures = new List<CaptureEntry>();
        }

        public static void Run()
        {
            var setup = Setup.FromCommandLine();
            ProjectSetup.Generate(setup);
            var input = CommandLine.GetRequiredArgument("-captureInput");
            var output = CommandLine.GetRequiredArgument("-captureOutput");
            var width = int.TryParse(
                CommandLine.GetOptionalArgument("-captureWidth"),
                out var parsedWidth
            )
                ? parsedWidth
                : DefaultWidth;
            var lines = File.ReadAllText(input).Replace("\r\n", "\n").TrimEnd('\n').Split('\n');
            Directory.CreateDirectory(output);
            var index = Render(setup, lines, width, output);
            File.WriteAllText(
                Path.Combine(output, "captures.json"),
                JsonUtility.ToJson(index, true)
            );
            WriteAssets(setup, Path.Combine(output, "assets"));
            Debug.Log($"Captured {index.captures.Count} lines into {output}");
        }

        private static CaptureIndex Render(
            Setup setup,
            IReadOnlyList<string> lines,
            int width,
            string output
        )
        {
            EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var font = AssetDatabase.LoadAssetAtPath<TMP_FontAsset>(
                $"{FontAssetBuilder.AssetFolder}/{setup.terminal}.asset"
            );
            var sprites = AssetDatabase.LoadAssetAtPath<TMP_SpriteAsset>(
                SpriteAssetBuilder.GetAssetPath(setup.sprites)
            );
            if (font == null || sprites == null)
            {
                throw new InvalidOperationException(
                    "the setup's terminal font or sprite asset was not generated"
                );
            }
            var target = new RenderTexture(width, MaxHeight, 24, RenderTextureFormat.ARGB32);
            var camera = new GameObject("Camera").AddComponent<Camera>();
            camera.orthographic = true;
            camera.clearFlags = CameraClearFlags.SolidColor;
            camera.backgroundColor = Color.black;
            camera.targetTexture = target;
            var canvas = new GameObject(
                "Canvas",
                typeof(RectTransform),
                typeof(Canvas),
                typeof(CanvasScaler)
            ).GetComponent<Canvas>();
            canvas.renderMode = RenderMode.ScreenSpaceCamera;
            canvas.worldCamera = camera;
            canvas.planeDistance = 1f;
            canvas.GetComponent<CanvasScaler>().uiScaleMode = CanvasScaler
                .ScaleMode
                .ConstantPixelSize;
            var terminal = Terminal.Create((RectTransform)canvas.transform, font, sprites, setup);
            var index = new CaptureIndex { width = width };
            var image = new Texture2D(width, MaxHeight, TextureFormat.RGBA32, false);
            for (var line = 0; line < lines.Count; line++)
            {
                terminal.Clear();
                terminal.Print(lines[line]);
                foreach (var text in terminal.Lines)
                {
                    text.ForceMeshUpdate();
                }
                Canvas.ForceUpdateCanvases();
                LayoutRebuilder.ForceRebuildLayoutImmediate(terminal.Content);
                Canvas.ForceUpdateCanvases();
                var height = Mathf.Min(MaxHeight, Mathf.CeilToInt(terminal.Content.rect.height));
                camera.Render();
                RenderTexture.active = target;
                image.ReadPixels(
                    new Rect(0, MaxHeight - height, width, height),
                    0,
                    MaxHeight - height
                );
                image.Apply();
                RenderTexture.active = null;
                var crop = new Texture2D(width, height, TextureFormat.RGBA32, false);
                crop.SetPixels(image.GetPixels(0, MaxHeight - height, width, height));
                var file = $"line-{line:000}.png";
                File.WriteAllBytes(Path.Combine(output, file), crop.EncodeToPNG());
                UnityEngine.Object.DestroyImmediate(crop);
                var entry = new CaptureEntry
                {
                    line = line,
                    file = file,
                    width = width,
                    height = height,
                };
                foreach (var text in terminal.Lines)
                {
                    entry.items.Add(Describe(text, terminal.Content));
                }
                index.captures.Add(entry);
            }
            UnityEngine.Object.DestroyImmediate(image);
            target.Release();
            return index;
        }

        private static ItemLayout Describe(TextMeshProUGUI text, RectTransform content)
        {
            var rect = text.rectTransform.rect;
            var corners = new Vector3[4];
            text.rectTransform.GetWorldCorners(corners);
            var top = content.InverseTransformPoint(corners[1]).y;
            var item = new ItemLayout
            {
                text = text.text,
                lossyScale = text.rectTransform.lossyScale.x,
                top = -top,
                width = rect.width,
                height = rect.height,
            };
            var info = text.textInfo;
            float ToRectX(float x) => x - rect.xMin;
            float ToRectY(float y) => rect.yMax - y;
            for (var i = 0; i < info.characterCount; i++)
            {
                var character = info.characterInfo[i];
                item.characters.Add(
                    new CharacterLayout
                    {
                        unicode = (int)character.character,
                        index = character.index,
                        sprite = character.elementType == TMP_TextElementType.Sprite,
                        visible = character.isVisible,
                        line = character.lineNumber,
                        origin = ToRectX(character.origin),
                        advance = ToRectX(character.xAdvance),
                        baseline = ToRectY(character.baseLine),
                        ascender = ToRectY(character.ascender),
                        descender = ToRectY(character.descender),
                        left = ToRectX(character.topLeft.x),
                        top = ToRectY(character.topLeft.y),
                        right = ToRectX(character.bottomRight.x),
                        bottom = ToRectY(character.bottomRight.y),
                        pointSize = character.pointSize,
                        scale = character.scale,
                    }
                );
            }
            for (var i = 0; i < info.lineCount; i++)
            {
                var line = info.lineInfo[i];
                item.lines.Add(
                    new LineLayout
                    {
                        first = line.firstCharacterIndex,
                        last = line.lastCharacterIndex,
                        ascender = ToRectY(line.ascender),
                        descender = ToRectY(line.descender),
                        baseline = ToRectY(line.baseline),
                        lineHeight = line.lineHeight,
                        width = line.width,
                        length = line.length,
                        marginLeft = line.marginLeft,
                        marginRight = line.marginRight,
                        alignment = (int)line.alignment,
                    }
                );
            }
            for (var m = 0; m < info.meshInfo.Length; m++)
            {
                var mesh = info.meshInfo[m];
                if (mesh.vertexCount == 0 || mesh.material == null)
                {
                    continue;
                }
                var layout = new MeshLayout
                {
                    material = mesh.material.name,
                    texture =
                        mesh.material.mainTexture != null ? mesh.material.mainTexture.name : "",
                };
                for (var v = 0; v < mesh.vertexCount; v++)
                {
                    layout.positions.Add(ToRectX(mesh.vertices[v].x));
                    layout.positions.Add(ToRectY(mesh.vertices[v].y));
                    layout.uvs.Add(mesh.uvs0[v].x);
                    layout.uvs.Add(mesh.uvs0[v].y);
                    layout.scales.Add(mesh.uvs2[v].y);
                    var color = mesh.colors32[v];
                    layout.colors.Add(color.r << 24 | color.g << 16 | color.b << 8 | color.a);
                }
                item.meshes.Add(layout);
            }
            return item;
        }

        private static void WriteAssets(Setup setup, string folder)
        {
            Directory.CreateDirectory(folder);
            var fonts = new List<TMP_FontAsset>();
            foreach (
                var guid in AssetDatabase.FindAssets(
                    "t:TMP_FontAsset",
                    new[] { "Assets/Generated" }
                )
            )
            {
                fonts.Add(
                    AssetDatabase.LoadAssetAtPath<TMP_FontAsset>(
                        AssetDatabase.GUIDToAssetPath(guid)
                    )
                );
            }
            foreach (var spec in setup.bundled)
            {
                var bundledFont = Resources.Load<TMP_FontAsset>("Fonts & Materials/" + spec.name);
                if (bundledFont != null)
                {
                    fonts.Add(bundledFont);
                }
            }
            foreach (var font in fonts)
            {
                var asset = new AssetLayout
                {
                    name = font.name,
                    material = font.material != null ? font.material.name : "",
                    atlasWidth = font.atlasWidth,
                    atlasHeight = font.atlasHeight,
                    padding = font.atlasPadding,
                };
                for (var a = 0; a < font.atlasTextures.Length && font.atlasTextures[a] != null; a++)
                {
                    var file = $"{font.name}.{a}.png";
                    File.WriteAllBytes(
                        Path.Combine(folder, file),
                        AtlasPng.ReadAlphaPng(font.atlasTextures[a])
                    );
                    asset.atlases.Add(file);
                }
                foreach (var character in font.characterTable)
                {
                    var rect = character.glyph.glyphRect;
                    asset.glyphs.Add(
                        new GlyphLayout
                        {
                            unicode = character.unicode,
                            x = rect.x,
                            y = rect.y,
                            width = rect.width,
                            height = rect.height,
                            atlas = character.glyph.atlasIndex,
                        }
                    );
                }
                File.WriteAllText(
                    Path.Combine(folder, font.name + ".json"),
                    JsonUtility.ToJson(asset, true)
                );
            }
            var spriteAsset = AssetDatabase.LoadAssetAtPath<TMP_SpriteAsset>(
                SpriteAssetBuilder.GetAssetPath(setup.sprites)
            );
            if (spriteAsset != null)
            {
                var sprites = new AssetLayout
                {
                    name = spriteAsset.name,
                    material = spriteAsset.material != null ? spriteAsset.material.name : "",
                    atlasWidth = spriteAsset.spriteSheet.width,
                    atlasHeight = spriteAsset.spriteSheet.height,
                };
                foreach (var glyph in spriteAsset.spriteGlyphTable)
                {
                    var rect = glyph.glyphRect;
                    sprites.glyphs.Add(
                        new GlyphLayout
                        {
                            unicode = glyph.index,
                            x = rect.x,
                            y = rect.y,
                            width = rect.width,
                            height = rect.height,
                        }
                    );
                }
                File.WriteAllText(
                    Path.Combine(folder, spriteAsset.name + ".json"),
                    JsonUtility.ToJson(sprites, true)
                );
            }
        }
    }
}
