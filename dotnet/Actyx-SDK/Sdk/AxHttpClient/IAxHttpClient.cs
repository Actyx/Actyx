
using System;
using System.Threading.Tasks;

namespace Actyx.Sdk.AxHttpClient
{
    public interface IAxHttpClient
    {
        Task<Res> Post<Req, Res>(string path, Req payload);
        Task<Res> Get<Res>(string path);
        IObservable<Res> Stream<Req, Res>(string path, Req payload);
    }
}
