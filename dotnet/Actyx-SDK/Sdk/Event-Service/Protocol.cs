using Actyx.Sdk.Utils.Extensions;
using JsonSubTypes;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Actyx
{
    public static class Proto<T>
    {
        public static T Deserialize(string json) => JsonConvert.DeserializeObject<T>(json, HttpContentExtensions.JsonSettings);
        public static string Serialize(T value) => JsonConvert.SerializeObject(value, HttpContentExtensions.JsonSettings);

    }

    [JsonConverter(typeof(JsonSubtypes), "Type")]
    public interface IErrorKind
    {
        public string Type { get; }
    }

    public class UnknownEndpoint : IErrorKind
    {
        public string Type { get; } = "unknownEndpoint";
        public string Endpoint { get; set; }
        public string[] ValidEndpoints { get; set; }
    }

    public class InternalError : IErrorKind
    {
        public string Type { get; } = "internalError";
    }

    public class BadRequest : IErrorKind
    {
        public string Type { get; } = "badRequest";
        public string Message { get; set; }
    }

    public class ServiceError : IErrorKind
    {
        public string Type { get; } = "serviceError";
        public JObject Value { get; set; }
    }

    [JsonConverter(typeof(JsonSubtypes), "Type")]
    public interface IOutgoing
    {
        public string Type { get; }
    }

    public class Request : IOutgoing
    {
        public string Type { get; } = "request";
        public string ServiceId { get; set; }
        public ulong RequestId { get; set; }
        public JObject Payload { get; set; }
    }
    public class Cancel : IOutgoing
    {
        public string Type { get; } = "cancel";
        public ulong RequestId { get; set; }
    }

    [JsonConverter(typeof(JsonSubtypes), "Type")]
    public interface IIncoming
    {
        public string Type { get; }
    }
    public class Next : IIncoming
    {
        public string Type { get; } = "next";
        public ulong RequestId { get; set; }
        public JObject[] Payload { get; set; }
    }
    public class Complete : IIncoming
    {
        public string Type { get; } = "complete";
        public ulong RequestId { get; set; }
    }
    public class Error : IIncoming
    {
        public string Type { get; } = "error";
        public ulong RequestId { get; set; }
        public IErrorKind Kind { get; set; }
    }
}
