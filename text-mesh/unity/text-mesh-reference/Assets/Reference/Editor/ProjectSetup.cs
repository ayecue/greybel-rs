using System.IO;
using System.Text;
using UnityEditor;
using UnityEngine;

namespace TextMeshReference.Editor
{
    public static class ProjectSetup
    {
        public const string ReportPath = "Logs/font-report.txt";

        public static void Run()
        {
            Generate(Setup.FromCommandLine());
        }

        public static void Generate(Setup setup)
        {
            PlayerSettings.colorSpace = ColorSpace.Gamma;
            AssetDatabase.DeleteAsset("Assets/Generated");
            var report = new StringBuilder();
            var fonts = FontAssetBuilder.Build(setup, report);
            SpriteAssetBuilder.Build(setup);
            TmpSetup.Apply(setup, fonts);
            report.AppendLine();
            report.AppendLine("Default style sheet:");
            report.AppendLine(TmpSetup.DescribeStyleSheet());
            Directory.CreateDirectory(Path.GetDirectoryName(ReportPath));
            File.WriteAllText(ReportPath, report.ToString());
            AssetDatabase.SaveAssets();
            Debug.Log(report.ToString());
        }
    }
}
