using System.Collections.Generic;
using Actyx.Sdk.Formats;

namespace Sdk.IntegrationTests.Helpers
{
    public static class Constants
    {
        public static readonly AppManifest TrialManifest = new()
        {
            AppId = "com.example.ax-http-client-tests",
            DisplayName = "ax http client tests",
            Version = typeof(Constants).Assembly.GetName().Version.ToString(),
        };
    }
}
