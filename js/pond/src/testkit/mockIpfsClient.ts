/* eslint-disable @typescript-eslint/no-explicit-any */

import { Observable, Subject } from 'rxjs'
import { SourceId } from '..'
import { IpfsClient, PubsubEnvelope, PubSubSubResult } from '../store/ipfsClient'
import { Link, lookup } from '../util'
import { augmentedPathOr, putToDag } from './ipfsDagTestTools'
import { log } from './loggers'

const defaultConfig: Config = { type: 'perfect', initialDag: {} }

export type FailureConfig = Readonly<{
  name: string
  pFailure: number
  maxDelayMs: number
}>

const after = <T>(delay: number, value: T): Observable<T> => Observable.of(value).delay(delay)

const flakify = <T>(config: FailureConfig): ((value: T) => Observable<T>) => {
  const { pFailure, maxDelayMs, name } = config
  return value => {
    const fail = pFailure > 0 && Math.random() < pFailure
    if (fail) {
      const msg = `injected failure ${name} p=${pFailure}`
      log.test.debug(msg)
      return Observable.throw(msg)
    } else if (maxDelayMs < 0) {
      return Observable.of(value)
    } else {
      const delay = Math.ceil(Math.random() * maxDelayMs)
      const msg = `delayed response ${name} delay=${delay}`
      log.test.debug(msg)
      return after(delay, value)
    }
  }
}

export const FailureConfig = {
  flakify,
}

const defaultFailureConfig: FailureConfig = {
  maxDelayMs: 20,
  pFailure: 0.1,
  name: '',
}

export type FlakyClientConfig = Readonly<{
  type: 'flaky'
  initialDag: { [key: string]: any }
  get: FailureConfig
  put: FailureConfig
  sub: FailureConfig
  pub: FailureConfig
}>

export const FlakyClientConfig = {
  default: (): FlakyClientConfig => ({
    type: 'flaky',
    initialDag: {},
    get: { ...defaultFailureConfig, name: 'get' },
    put: { ...defaultFailureConfig, name: 'put' },
    sub: { ...defaultFailureConfig, name: 'sub' },
    pub: { ...defaultFailureConfig, name: 'pub' },
  }),
}

export type PerfectClientConfig = Readonly<{
  type: 'perfect'
  initialDag: { [key: string]: any }
}>

export type NoClientConfig = Readonly<{
  type: 'none'
}>

export type Config = PerfectClientConfig | NoClientConfig | FlakyClientConfig

export const Config = {
  default: defaultConfig,
}

type Self = Readonly<{
  id: string
  sub: (topic: string) => Observable<PubsubEnvelope<any>>
  pub: (topic: string, env: PubsubEnvelope<any>) => Observable<void>
  getDag: () => Observable<any>
  modDag: (f: (dag: any) => [string, any]) => Observable<string>
}>

const dagPut = (self: Self) => <T>(value: T): Observable<Link<T>> =>
  self.modDag(dag => putToDag(dag, value)).map(hash => Link.of(hash))

const dagGet = (self: Self) => <T>(link: Link<T>): Observable<T> =>
  self.getDag().map(dag => {
    const cid = link['/']
    const path = cid.split('/')
    const data = augmentedPathOr(path, dag)
    if (data === undefined) {
      throw new Error(`path at ${cid} not found!`)
    }
    return data
  })

const pubsubSub = (self: Self) => (topic: string): Observable<PubsubEnvelope<any>> =>
  self.sub(topic).catch(() => Observable.never<PubsubEnvelope<any>>())

const pubsubPub = (self: Self) => (topic: string, data: any): Observable<PubSubSubResult> => {
  const env: PubsubEnvelope<any> = {
    from: self.id,
    data,
    seqno: 'seqno',
    topicIDs: [topic],
  }
  return self
    .pub(topic, env)
    .mapTo({ success: true, msg: 'OK' })
    .catch(error => Observable.of({ success: true, msg: error }))
}

const mkMockIpfsClient = (
  id: string,
  getDag: () => Observable<any>,
  modDag: (f: (dag: any) => [string, any]) => Observable<string>,
  sub: (topic: string) => Observable<PubsubEnvelope<any>>,
  pub: (topic: string, msg: PubsubEnvelope<any>) => Observable<any>,
): IpfsClient => {
  // todo: mock gc
  const self: Self = { id, getDag, modDag, pub, sub }
  return {
    dag: {
      put: dagPut(self),
      get: dagGet(self),
    },
    pubsub: {
      pub: pubsubPub(self),
      sub: pubsubSub(self),
    },
  }
}

const mkNoIpfsClient = () => {
  const id = SourceId.random(10)
  const getDag = () => Observable.throw('noboby home')
  const modDag = (_dag: any) => Observable.throw('nobody there')
  const sub = (_topic: string) => Observable.throw('Boom')
  const pub = (_topic: string, _msg: PubsubEnvelope<any>) => Observable.throw('nope, still nothing')
  return mkMockIpfsClient(id, getDag, modDag, sub, pub)
}

const mkDag = (initialDag: any) => {
  let dag: any = initialDag
  const get = () => Observable.of(dag)
  const mod = (f: (dag: any) => [any, string]) => {
    const [dag1, hash] = f(dag)
    dag = dag1
    return Observable.of(hash)
  }
  return { get, mod }
}

const mkPubSub = () => {
  const topics: { [key: string]: Subject<PubsubEnvelope<any>> | undefined } = {}
  const getOrCreateTopic = (topic: string): Subject<PubsubEnvelope<any>> => {
    const result = lookup(topics, topic)
    if (result === undefined) {
      const result1 = new Subject<PubsubEnvelope<any>>()
      topics[topic] = result1
      return result1
    } else {
      return result
    }
  }
  const sub = (topic: string): Observable<PubsubEnvelope<any>> => getOrCreateTopic(topic)
  const pub = (topic: string, msg: PubsubEnvelope<any>): Observable<void> => {
    getOrCreateTopic(topic).next(msg)
    return Observable.of(undefined)
  }
  return { pub, sub }
}

const mkFlakyIpfsClient = (config: FlakyClientConfig) => {
  const id = SourceId.random(10)
  const { get, mod } = mkDag(config.initialDag)
  const { pub, sub } = mkPubSub()
  const getf = flakify<any>(config.get)
  // note: for put we would have to flakify both before and after
  // but since it is cas it does not really matter all that much
  const modf = flakify<string>(config.put)
  const subf = flakify<PubsubEnvelope<any>>(config.sub)
  const pubf = flakify<void>(config.pub)
  return mkMockIpfsClient(
    id,
    () => get().flatMap(getf),
    (dag: any) => mod(dag).flatMap(modf),
    (topic: string) =>
      sub(topic)
        .concatMap(subf)
        .retryWhen(errors => errors.delay(1000))
        .repeatWhen(errors => errors.delay(1000)),
    (topic: string, msg: any) => pub(topic, msg).flatMap(pubf),
  )
}

const mkPerfectIpfsClient = (config: PerfectClientConfig) => {
  const id = SourceId.random(10)
  const { get, mod } = mkDag(config.initialDag)
  const { pub, sub } = mkPubSub()
  return mkMockIpfsClient(id, get, mod, sub, pub)
}

const clientFromConfig = (config: Config): IpfsClient => {
  switch (config.type) {
    case 'perfect':
      return mkPerfectIpfsClient(config)
    case 'none':
      return mkNoIpfsClient()
    case 'flaky':
      return mkFlakyIpfsClient(config)
  }
}

export const MockIpfsClient = {
  of: clientFromConfig,
}

/*
  This client can do a bit more, it has additional facility for tracing the count of gets and puts performed.
  One example usage is integration testing of caching facility (e.g. of observableCache - see maxBlockLengthCompaction.spec.ts and possibly others)
  Please note that put statistics are per hash, whereas get statistics are per key equal to link['/']. This is purposeful,
  as cache functions exactly this way, it does not cache reads from particular hash, instead it caches accesses to a particular link.
  See e.g. observableCache.ts
*/

export type IpfsClientTracer = Readonly<{
  countGet: (key: string) => number
  countPut: (key: string) => number
}>

export type TracingIpfsClient = Readonly<{
  client: IpfsClient
  tracer: IpfsClientTracer
}>

type DagOperationTracer = Readonly<{
  putTracer: { [key: string]: number } // how many times a write happened for the key = to a given hash
  getTracer: { [key: string]: number } // how many times a read has been executed for the key = given link['/'] ?
}>

const tracedDagPut = (self: Self, tracer: DagOperationTracer) => <T>(
  value: T,
): Observable<Link<T>> =>
  self
    .modDag(dag => putToDag(dag, value))
    .do(
      // please note that this is a data race, but I don't envision a full async usage atm
      hash => (tracer.putTracer[hash] = (tracer.putTracer[hash] || 0) + 1),
    )
    .map(hash => Link.of(hash))

const tracedDagGet = (self: Self, tracer: DagOperationTracer) => <T>(
  link: Link<T>,
): Observable<T> => {
  const key = link['/']
  tracer.getTracer[key] = (tracer.getTracer[key] || 0) + 1
  return self.getDag().map(dag => {
    const cid = link['/']
    const path = cid.split('/')
    const data = augmentedPathOr(path, dag)
    if (data === undefined) {
      throw new Error(`path at ${cid} not found!`)
    }
    return data
  })
}

const mkMockTracingIpfsClient = (
  id: string,
  getDag: () => Observable<any>,
  modDag: (f: (dag: any) => [string, any]) => Observable<string>,
  sub: (topic: string) => Observable<PubsubEnvelope<any>>,
  pub: (topic: string, msg: PubsubEnvelope<any>) => Observable<any>,
): TracingIpfsClient => {
  const tracer: DagOperationTracer = { getTracer: {}, putTracer: {} }
  const self: Self = { id, getDag, modDag, pub, sub }
  return {
    client: {
      dag: {
        put: tracedDagPut(self, tracer),
        get: tracedDagGet(self, tracer),
      },
      pubsub: {
        pub: pubsubPub(self),
        sub: pubsubSub(self),
      },
    },
    tracer: {
      countGet: (key: string) => tracer.getTracer[key] || 0,
      countPut: (key: string) => tracer.putTracer[key] || 0,
    },
  }
}

const mkPerfectTracingIpfsClient = (config: PerfectClientConfig) => {
  const id = SourceId.random(10)
  const { get, mod } = mkDag(config.initialDag)
  const { pub, sub } = mkPubSub()
  return mkMockTracingIpfsClient(id, get, mod, sub, pub)
}

export const MockPerfectTracingIpfsClient = {
  of: (config: PerfectClientConfig) => mkPerfectTracingIpfsClient(config),
}
