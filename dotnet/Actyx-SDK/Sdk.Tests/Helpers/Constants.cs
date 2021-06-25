using System;
using System.Collections.Generic;
using Actyx.Sdk.Formats;

namespace Sdk.Tests.Helpers
{
    public static class Constants
    {
        public static readonly AppManifest TrialManifest = new()
        {
            AppId = "com.example.ax-http-client-tests",
            DisplayName = "ax http client tests",
            Version = "1.0.0"
        };

        public static IEnumerable<string> Tags => new List<string>() { "42", "order", "dotnet" };

        public static string ApiOrigin => Environment.GetEnvironmentVariable("HTTP_API_ORIGIN") ?? "http://localhost:4454";
    }
}
