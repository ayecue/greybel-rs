using System;
using System.Linq;

namespace GreyHackMessageHook
{
    public static class BuildHelper
    {
        public const int TemporaryFileNameLength = 5;
        static readonly string RandomStringChars =
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

        public static string GenerateRandomString(int length)
        {
            var random = new Random();
            return new string(
                Enumerable
                    .Repeat(RandomStringChars, length)
                    .Select(s => s[random.Next(s.Length)])
                    .ToArray()
            );
        }
    }
}
