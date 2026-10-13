using System;
using System.IO;

namespace GreyHackMessageHook
{
    public class FilePath
    {
        public string Path;
        public string Extension;

        private static bool IsSupportedFileExtension(string extension)
        {
            string[] entries = Plugin.FileExtensions.Value.Split(",");

            foreach (var entry in entries)
            {
                if (
                    string.Equals(extension, "." + entry.Trim(), StringComparison.OrdinalIgnoreCase)
                )
                {
                    return true;
                }
            }

            return false;
        }

        private static string GetPathWithoutExtension(string path)
        {
            string extension = PosixPath.GetExtension(path);

            if (IsSupportedFileExtension(extension))
            {
                return PosixPath.Combine(
                    PosixPath.GetDirectoryName(path) ?? "",
                    PosixPath.GetFileNameWithoutExtension(path)
                );
            }

            return path;
        }

        private static string GetPathExtension(string path)
        {
            string extension = PosixPath.GetExtension(path);

            if (IsSupportedFileExtension(extension))
            {
                return extension;
            }

            return "";
        }

        public static FilePath ResolveFilePath(string basePath, string source, string target)
        {
            string origin = PosixPath.GetDirectoryName(source);
            if (target.StartsWith("/"))
            {
                origin = basePath;
                target = "." + target;
            }
            string path = PosixPath.Combine(origin, target);
            string fullPath = PosixPath.GetFullPath(path);
            return new FilePath(fullPath);
        }

        public FilePath(string fullPath)
        {
            Path = GetPathWithoutExtension(fullPath);
            Extension = GetPathExtension(fullPath);
        }

        public FilePath(string path, string extension = null)
        {
            this.Path = path;
            this.Extension = extension;
        }

        public string ToFullPath()
        {
            return Path + Extension;
        }
    }
}
