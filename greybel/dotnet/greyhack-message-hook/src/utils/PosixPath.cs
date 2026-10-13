using System.IO;
using System.Linq;

namespace GreyHackMessageHook
{
    public static class PosixPath
    {
        public static string Join(string left, string right)
        {
            return Path.Join(left, right).Replace('\\', '/');
        }

        public static string Combine(params string[] paths)
        {
            return Path.Combine(paths).Replace('\\', '/');
        }

        public static string GetDirectoryName(string path)
        {
            return Path.GetDirectoryName(path).Replace('\\', '/');
        }

        public static string GetFileName(string path)
        {
            return Path.GetFileName(path);
        }

        public static string GetExtension(string path)
        {
            return Path.GetExtension(path);
        }

        public static string GetFileNameWithoutExtension(string path)
        {
            return Path.GetFileNameWithoutExtension(path);
        }

        public static string GetFullPath(string path)
        {
            return Path.GetFullPath(path).Replace('\\', '/');
        }
    }
}
