using System;

namespace Actyx.Sdk.Utils
{
    internal static class ThrowIf
    {
        public static class Argument
        {
            public static void IsNull<T>(T argument, string name)
            {
                if (argument is null)
                {
                    throw new ArgumentNullException(name);
                }
            }
        }
    }
}
