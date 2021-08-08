using System;
using System.Reactive.Linq;

namespace Actyx.Sdk.Utils.Extensions
{
    internal static class ObservableExtensions
    {
        public static IObservable<R> TrySelect<TSource, R>(this IObservable<TSource> source, Func<TSource, R> selector, Action<TSource, Exception> onError = null) =>
            source.SelectMany(t =>
            {
                try { return Observable.Return(selector(t)); }
                catch (Exception e)
                {
                    if (!(onError is null)) { onError(t, e); }
                    return Observable.Empty<R>();
                }
            });
    }
}
