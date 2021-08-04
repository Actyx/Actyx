using System.Collections.Generic;
using Actyx.Sdk.Utils;
using JsonSubTypes;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Actyx.Sdk.Wsrpc
{
    public static class WsrpcSerializer
    {
        public static JsonSerializer Create(bool pretty = true) =>
            DefaultJsonSerializer.Create(new List<JsonConverter>
            {
                JsonSubtypesConverterBuilder
                    .Of(typeof(IResponseMessage), "type")
                    .RegisterSubtype<Next>("next")
                    .RegisterSubtype<Complete>("complete")
                    .RegisterSubtype<Error>("error")
                    .SerializeDiscriminatorProperty()
                    .Build(),
                JsonSubtypesConverterBuilder
                    .Of(typeof(IRequestMessage), "type")
                    .RegisterSubtype<Request>("request")
                    .RegisterSubtype<Cancel>("cancel")
                    .SerializeDiscriminatorProperty()
                    .Build(),
                JsonSubtypesConverterBuilder
                    .Of(typeof(IErrorKind), "type")
                    .RegisterSubtype<UnknownEndpoint>("unknownEndpoint")
                    .RegisterSubtype<InternalError>("internalError")
                    .RegisterSubtype<BadRequest>("badRequest")
                    .RegisterSubtype<ServiceError>("serviceError")
                    .SerializeDiscriminatorProperty()
                    .Build(),
            }, pretty);
    }

    public interface IErrorKind { }

    public class UnknownEndpoint : IErrorKind
    {
        public string Endpoint { get; set; }
        public string[] ValidEndpoints { get; set; }
    }

    public class InternalError : IErrorKind { }

    public class BadRequest : IErrorKind
    {
        public string Message { get; set; }
    }

    public class ServiceError : IErrorKind
    {
        public JToken Value { get; set; }
    }

    public interface IRequestMessage
    {
        public long RequestId { get; }
    }

    public class Request : IRequestMessage
    {
        public string ServiceId { get; set; }
        public long RequestId { get; set; }
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public JToken Payload { get; set; }
    }
    public class Cancel : IRequestMessage
    {
        public long RequestId { get; set; }
    }

    public interface IResponseMessage
    {
        public long RequestId { get; }
    }
    public class Next : IResponseMessage
    {
        public long RequestId { get; set; }
        public JToken[] Payload { get; set; }
    }
    public class Complete : IResponseMessage
    {
        public long RequestId { get; set; }
    }
    public class Error : IResponseMessage
    {
        public long RequestId { get; set; }
        public IErrorKind Kind { get; set; }
    }
}
