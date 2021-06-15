using JsonSubTypes;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using Newtonsoft.Json.Serialization;

namespace Actyx
{

    [JsonConverter(typeof(JsonSubtypes), "Type")]
    public interface IErrorKind
    {
        public string Type();
    }

    public class UnknownEndpoint : IErrorKind
    {
        public string Type() => "unknownEndpoint";
        public string Endpoint { get; set; }
        public string[] ValidEndpoints { get; set; }
    }

    public class InternalError : IErrorKind
    {
        public string Type() => "internalError";
    }

    public class BadRequest : IErrorKind
    {
        public string Type() => "badRequest";
        public string Message { get; set; }
    }

    public class ServiceError : IErrorKind
    {
        public string Type() => "serviceError";
        public JObject Value { get; set; }
    }

    [JsonConverter(typeof(JsonSubtypes), "Type")]
    public abstract class Outgoing
    {
        static readonly JsonSerializerSettings jsonSerializerSettings;
        static Outgoing()
        {
            DefaultContractResolver contractResolver = new DefaultContractResolver
            {
                NamingStrategy = new CamelCaseNamingStrategy()
            };
            jsonSerializerSettings = new JsonSerializerSettings
            {
                ContractResolver = contractResolver,
            };
        }
        public static Outgoing Deserialize(string json) => JsonConvert.DeserializeObject<Outgoing>(json, jsonSerializerSettings);
        public static string Serialize(Outgoing outgoing) => JsonConvert.SerializeObject(outgoing, jsonSerializerSettings);
        public abstract string Type();
    }

    public class Request : Outgoing
    {
        override public string Type() => "request";
        public string ServiceId { get; set; }
        public ulong RequestId { get; set; }
        public JObject Payload { get; set; }
    }
    public class Cancel : Outgoing
    {
        override public string Type() => "cancel";
        public ulong RequestId { get; set; }
    }

    [JsonConverter(typeof(JsonSubtypes), "Type")]
    public abstract class Incoming
    {
        static readonly JsonSerializerSettings jsonSerializerSettings;
        static Incoming()
        {
            DefaultContractResolver contractResolver = new DefaultContractResolver
            {
                NamingStrategy = new CamelCaseNamingStrategy()
            };
            jsonSerializerSettings = new JsonSerializerSettings
            {
                ContractResolver = contractResolver,
            };
        }
        public static Incoming Deserialize(string json) => JsonConvert.DeserializeObject<Incoming>(json, jsonSerializerSettings);
        public static string Serialize(Incoming incoming) => JsonConvert.SerializeObject(incoming, jsonSerializerSettings);
        public abstract string Type();
    }
    public class Next : Incoming
    {
        override public string Type() => "next";
        public ulong RequestId { get; set; }
        public JObject Payload { get; set; }
    }
    public class Complete : Incoming
    {
        override public string Type() => "complete";
        public ulong RequestId { get; set; }
    }
    public class Error : Incoming
    {
        override public string Type() => "error";
        public ulong RequestId { get; set; }
        public IErrorKind Kind { get; set; }
    }


}
