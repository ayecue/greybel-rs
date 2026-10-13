using AssetsTools.NET;
using AssetsTools.NET.Extra;

namespace GreyHackUtil.Assets;

internal sealed record GameAsset(
    AssetsFileInstance File,
    AssetFileInfo Info,
    AssetTypeValueField Fields
)
{
    public string Name => Fields["m_Name"].AsString;
}

internal sealed class GameData
{
    private static readonly string[] SerializedFiles =
    [
        "globalgamemanagers",
        "globalgamemanagers.assets",
        "resources.assets",
        "sharedassets0.assets",
        "sharedassets1.assets",
        "level0",
        "level1",
    ];

    private readonly AssetsManager manager = new();
    private readonly List<AssetsFileInstance> files = [];
    private readonly Dictionary<(string File, long PathId), string> classes = [];

    public IReadOnlyList<(string Path, GameAsset? Asset)> Resources { get; }

    public GameData(string folder, string cache)
    {
        manager.MonoTempGenerator = new MonoCecilTempGenerator(Path.Combine(folder, "Managed"));
        foreach (string name in SerializedFiles)
        {
            string path = Path.Combine(folder, name);
            if (File.Exists(path))
            {
                files.Add(manager.LoadAssetsFile(path, loadDeps: false));
            }
        }
        string version = files[0].file.Metadata.UnityVersion;
        manager.LoadClassPackage(ClassPackage.Build(cache, version));
        manager.LoadClassDatabaseFromPackage(version);
        foreach (AssetsFileInstance file in files)
        {
            foreach (AssetFileInfo info in file.file.GetAssetsOfType(AssetClassID.MonoBehaviour))
            {
                AssetTypeValueField fields = manager.GetBaseField(
                    file,
                    info,
                    AssetReadFlags.SkipMonoBehaviourFields
                );
                GameAsset? script = Dereference(file, fields["m_Script"]);
                if (script != null)
                {
                    classes[(file.name, info.PathId)] = script.Fields["m_ClassName"].AsString;
                }
            }
        }
        AssetsFileInstance globals = files.Single(file => file.name == "globalgamemanagers");
        AssetFileInfo resourceManager = globals
            .file.GetAssetsOfType(AssetClassID.ResourceManager)
            .Single();
        Resources = manager
            .GetBaseField(globals, resourceManager)["m_Container.Array"]
            .Children.Select(pair => (pair["first"].AsString, Dereference(globals, pair["second"])))
            .ToList();
    }

    public GameAsset? Dereference(AssetsFileInstance file, AssetTypeValueField pointer)
    {
        if (pointer["m_PathID"].AsLong == 0)
        {
            return null;
        }
        AssetExternal external = manager.GetExtAsset(file, pointer);
        return external.info == null
            ? null
            : new GameAsset(external.file, external.info, external.baseField);
    }

    public GameAsset? Dereference(GameAsset source, AssetTypeValueField pointer) =>
        Dereference(source.File, pointer);

    public string? GetClass(GameAsset asset) =>
        classes.GetValueOrDefault((asset.File.name, asset.Info.PathId));

    public IEnumerable<GameAsset> GetAssetsOfClass(string name)
    {
        foreach (var ((fileName, pathId), className) in classes)
        {
            if (className != name)
            {
                continue;
            }
            AssetsFileInstance file = files.Single(candidate => candidate.name == fileName);
            AssetFileInfo info = file.file.GetAssetInfo(pathId);
            yield return new GameAsset(file, info, manager.GetBaseField(file, info));
        }
    }

    public IEnumerable<GameAsset> GetResources(string path) =>
        Resources
            .Where(resource => resource.Path == path && resource.Asset != null)
            .Select(resource => resource.Asset!);

    public (int Width, int Height, byte[] Bgra) ReadPixels(GameAsset texture)
    {
        var file = AssetsTools.NET.Texture.TextureFile.ReadTextureFile(texture.Fields);
        file.FillPictureData(texture.File);
        byte[] bottomUp = file.DecodeTextureRaw(file.pictureData);
        int width = file.m_Width;
        int height = file.m_Height;
        int stride = width * 4;
        byte[] topDown = new byte[bottomUp.Length];
        for (int row = 0; row < height; row++)
        {
            Buffer.BlockCopy(bottomUp, row * stride, topDown, (height - 1 - row) * stride, stride);
        }
        return (width, height, topDown);
    }
}
