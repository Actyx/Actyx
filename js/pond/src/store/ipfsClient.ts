/* eslint-disable @typescript-eslint/no-explicit-any */
import { Observable } from 'rxjs'
import { Link } from '../util'

export type PubSubSubResult = Readonly<{ success: boolean; msg: any }>
export type DagPutResult = { success: true; msg: { Cid: string } } | { success: false; msg: any }

export type PubsubEnvelope<T> = Readonly<{
  from: string
  data: T
  seqno: string
  topicIDs: ReadonlyArray<string>
}>

export type DagGet = <T>(link: Link<T>) => Observable<T>
export type DagPut = <T>(value: T) => Observable<Link<T>>

export type DagApi = Readonly<{
  get: DagGet
  put: DagPut
}>

/**
 * Publish a message on a topic.
 */
export type PubSubPub = (topic: string, data: any) => Observable<PubSubSubResult>

/**
 * Subscribe to the given topic and return all pubsub envelopes
 *
 * Implementers should take care of reconnect themselves,
 * and never fail this observable.
 */
export type PubSubSub = (topic: string) => Observable<PubsubEnvelope<any>>

export type PubSubApi = Readonly<{
  sub: PubSubSub
  pub: PubSubPub
}>

export interface IpfsClient {
  readonly dag: DagApi
  readonly pubsub: PubSubApi
}
