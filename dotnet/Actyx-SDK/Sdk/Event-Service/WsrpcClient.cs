using System;
using System.Collections.Generic;
using System.Reactive.Disposables;
using System.Reactive.Linq;
using Newtonsoft.Json.Linq;
using Websocket.Client;

namespace Actyx
{
    public class WsrpcClient : IDisposable
    {
        private readonly WebsocketClient client;
        private readonly Dictionary<ulong, IObserver<IResponseMessage>> listeners = new() { };
        private readonly IDisposable responseProcessor;
        private Exception error;
        private ulong requestCounter = 0;

        public class Error : Exception
        {
            private readonly IErrorKind errorKind;
            public Error(IErrorKind errorKind) : base(Proto<IErrorKind>.Serialize(errorKind))
            {
                this.errorKind = errorKind;
            }
        }

        public WsrpcClient(string token)
        {
            client = new WebsocketClient(new Uri($"ws://localhost:4454/api/v2/events?{token}"))
            {
                ReconnectTimeout = TimeSpan.FromMinutes(5)
            };
            client.ReconnectionHappened.Subscribe(info =>
                Console.WriteLine($"Reconnection happened, type: {info.Type}"));

            responseProcessor = client.MessageReceived.Subscribe(msg =>
            {
                var response = Proto<IResponseMessage>.Deserialize(msg.Text);
                var listener = listeners[response.RequestId];
                if (!(listener is null))
                {
                    listener.OnNext(response);
                }
                else
                {
                    Console.WriteLine($"No listener registered for message {response}");
                }
            }, err =>
            {
                Console.WriteLine($"response processor error: {err}");
                ClearListeners(l => l.OnCompleted());
                error = err;
            });
        }

        public IObservable<JToken> Request(string serviceId, JToken payload)
        {
            if (!(error is null)) return (IObservable<JToken>)Observable.Throw<Exception>(error);

            var upstreamCompletedOnError = false;
            return Multiplex(serviceId, payload, () => !upstreamCompletedOnError)
                .TakeWhile(res =>
                {
                    var isComplete = res.Type == "complete";
                    if (isComplete) upstreamCompletedOnError = true;
                    return !isComplete;
                }).SelectMany(res =>
                {
                    switch (res)
                    {
                        case Next next: return next.Payload;
                        case Actyx.Error error:
                            {
                                upstreamCompletedOnError = true;
                                throw new Error(error.Kind);
                            }
                        default: throw new InvalidOperationException();
                    }
                });
        }

        IObservable<IResponseMessage> Multiplex(string serviceId, JToken payload, Func<bool> shouldCancelUpstream)
        {
            var requestId = requestCounter++;
            var (request, cancel) = Handlers(requestId, serviceId, payload);
            var res = Observable.Create((IObserver<IResponseMessage> observer) =>
            {
                listeners[requestId] = observer;
                return Disposable.Create(() =>
                {
                    try
                    {
                        Console.WriteLine($"About to unsubscribe from requestId: {requestId}");

                        if (shouldCancelUpstream())
                        {
                            Console.WriteLine($"About to unsubscribe with: {cancel}");
                            var cancelMsg = Proto<Cancel>.Serialize(cancel);
                            client.Send(cancelMsg);
                        }
                        else
                        {
                            Console.WriteLine($"RequestId {requestId} was cancelled by upstream, not sending a cancelMsg");
                        }
                    }
                    catch (Exception err)
                    {
                        Console.WriteLine($"Unsubscribe error {err}");
                        observer.OnError(err);
                    }
                    listeners.Remove(requestId);
                });
            });
            try
            {
                Console.WriteLine($"About to subscribe {request}");
                var requestMsg = Proto<Request>.Serialize(request);
                client.Send(requestMsg);
            }
            catch (Exception err)
            {
                return (IObservable<IResponseMessage>)Observable.Throw<Exception>(err);
            }
            return res;
        }

        (Request, Cancel) Handlers(ulong requestId, string serviceId, JToken payload)
        {
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
