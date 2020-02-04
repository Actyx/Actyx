import * as t from 'io-ts'
import { Observable } from 'rxjs'
import { IpfsClient, Link } from '.'
import { MultiplexedWebsocket, validateOrThrow } from './eventstore/multiplexedWebsocket'
import { DagCbor } from './store/dagCborUtil'
import { DagApi, PubSubApi } from './store/ipfsClient'

const enum RequestTypes {
  IpfsBlockPut = '/ax/ipfs/blockPut',
  IpfsBlockGet = '/ax/ipfs/blockGet',
}

const dagPutAsBlock = (m: MultiplexedWebsocket) => <T>(value: T): Observable<Link<T>> =>
  DagCbor.encode(value)
    .map(buffer => buffer.toString('base64'))
    .concatMap(text => m.request(RequestTypes.IpfsBlockPut, text).take(1))
    .map(validateOrThrow(t.string))
    .map(hash => Link.of<T>(hash))

const dagGetAsBlock = (m: MultiplexedWebsocket) => <T>(link: Link<T>): Observable<T> =>
  m
    .request(RequestTypes.IpfsBlockGet, link['/'])
    .take(1)
    .map(validateOrThrow(t.string))
    .concatMap<string, T>(base64 => DagCbor.decode(Buffer.from(base64, 'base64')))

const mkDagApi = (m: MultiplexedWebsocket): DagApi => ({
  put: dagPutAsBlock(m),
  get: dagGetAsBlock(m),
})

// will send into nirvana and never produce messages
const noopPubSubApi: PubSubApi = {
  pub: () => Observable.of({ success: true, msg: 'OK' }),
  sub: () => Observable.never(),
}

export class WebsocketIpfsClient implements IpfsClient {
  readonly dag: DagApi
  readonly pubsub: PubSubApi
  constructor(multiplexer: MultiplexedWebsocket) {
    this.dag = mkDagApi(multiplexer)
    this.pubsub = noopPubSubApi
  }
}
