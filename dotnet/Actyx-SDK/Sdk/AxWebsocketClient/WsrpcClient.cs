using System;
using System.Collections.Concurrent;
using System.Reactive.Disposables;
using System.Reactive.Linq;
using System.Threading;
using Newtonsoft.Json.Linq;
using Websocket.Client;

namespace Actyx.Sdk.AxWebsocketClient
{
    public class WsrpcClient : IDisposable
    {
        private readonly WebsocketClient client;
        private readonly ConcurrentDictionary<long, IObserver<IResponseMessage>> listeners = new() { };
        private readonly IDisposable responseProcessor;
        private Exception error;
        private long requestCounter = -1;

        public class Error : Exception
        {
            private readonly IErrorKind errorKind;
            public Error(IErrorKind errorKind) : base(Proto<IErrorKind>.Serialize(errorKind))
            {
                this.errorKind = errorKind;
            }
        }

        public WsrpcClient(Uri uri)
        {
            client = new WebsocketClient(uri)
            {
                ReconnectTimeout = TimeSpan.FromMinutes(5)
            };

            responseProcessor = client.MessageReceived.Subscribe(msg =>
            {
                var response = Proto<IResponseMessage>.Deserialize(msg.Text);
                if (listeners.TryGetValue(response.RequestId, out IObserver<IResponseMessage> listener))
                {
                    listener.OnNext(response);
                }
            }, err =>
            {
                ClearListeners(l => l.OnError(err));
                error = err;
            });
        }

        public IObservable<JToken> Request(string serviceId, JToken payload)
        {
            if (!(error is null)) { return (IObservable<JToken>)Observable.Throw<Exception>(error); }

            var upstreamCompletedOrError = false;
            return Multiplex(serviceId, payload, () => !upstreamCompletedOrError)
                .TakeWhile(res =>
                {
                    var isComplete = res.Type == "complete";
                    if (isComplete) { upstreamCompletedOrError = true; }
                    return !isComplete;
                }).SelectMany(res =>
                {
                    switch (res)
                    {
                        case Next next: return next.Payload;
                        case AxWebsocketClient.Error error:
                            {
                                upstreamCompletedOrError = true;
                                throw new Error(error.Kind);
                            }
                        default: throw new InvalidOperationException();
                    }
                });
        }

        IObservable<IResponseMessage> Multiplex(string serviceId, JToken payload, Func<bool> shouldCancelUpstream)
        {
            var requestId = Interlocked.Increment(ref requestCounter);
            var (request, cancel) = Handlers(requestId, serviceId, payload);
            var res = Observable.Create((IObserver<IResponseMessage> observer) =>
            {
                listeners[requestId] = observer;
                return Disposable.Create(() =>
                {
                    try
                    {
                        if (shouldCancelUpstream())
                        {
                            var cancelMsg = Proto<Cancel>.Serialize(cancel);
                            client.Send(cancelMsg);
                        }
                    }
                    catch (Exception err)
                    {
                        observer.OnError(err);
                    }
                    listeners.TryRemove(requestId, out var _);
                });
            });

            try
            {
                var requestMsg = Proto<Request>.Serialize(request);
                client.Send(requestMsg);
            }
            catch (Exception err)
            {
                return (IObservable<IResponseMessage>)Observable.Throw<Exception>(err);
            }
            return res;
        }

        (Request, Cancel) Handlers(long requestId, string serviceId, JToken payload)
        {
            if (string.IsNullOrEmpty(serviceId)) { throw new ArgumentException($"'{nameof(serviceId)}' cannot be null or empty.", nameof(serviceId)); }
            return (
                new Request { ServiceId = serviceId, RequestId = requestId, Payload = payload },
                new Cancel { RequestId = requestId }
            );
        }

        void ClearListeners(Action<IObserver<IResponseMessage>> action)
        {
            foreach (var listener in listeners.Values)
            {
                action(listener);
            }
            listeners.Clear();
        }

        public void Dispose()
        {
            responseProcessor.Dispose();
            client.Dispose();
            ClearListeners(l => l.OnCompleted());
        }

        public void Start()
        {
            client.Start();
        }
    }
}
