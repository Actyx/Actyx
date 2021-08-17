using System.Linq;

namespace Actyx.Sdk.Utils
{
    public static class AxRandom
    {
        private static readonly System.Random random = new();
        public static string String(int length)
        {
            const string chars = "abcdefghijklmnopqrstuvwxyz";
            return new string(Enumerable.Repeat(chars, length)
              .Select(s => s[random.Next(s.Length)]).ToArray());
        }
    }
}
