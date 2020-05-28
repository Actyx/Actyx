/*
 * Copyright 2020 Actyx AG
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

/**
 * This is a type alias representing an offset map. It maps _Source IDs_ to
 * offsets for those _Source IDs_.
 */
export interface OffsetMap {
  [source: string]: number
}

/**
 * Enumeration of the ordering options you may use when querying the Event
 * Service. This type is use in [[QueryOpts]].
 */
export enum Ordering {
  Lamport = 'lamport',
  LamportReverse = 'lamport-reverse',
  SourceOrdered = 'source-ordered',
}

/**
 * Identifier for a stream. In ActyxOS each stream is defined by a semantic, a
 * name and a source.
 */
export interface StreamIdentifier {
  streamSemantics: string
  streamName: string
  source: string
}

/**
 * Type definition for events as returned by the ActyxOS Event Service API.
 *
 * _Note that the SDK does not provide any functionality for specifying event
 * payload types. This should be done at the application level._
 */
export interface Event {
  stream: StreamIdentifier
  timestamp: number
  lamport: number
  offset: number
  payload: unknown
}

/**
 * An event draft contains all the information necessary for publishing an
 * event. It is referred to as a draft since it becomes an actual event when
 * it is successfully published by the Event Service.
 *
 * _Note you never specify a Source ID for an event draft since ActyxOS will
 * automatically attach the publishing node's Source ID to each event that it
 * publishes._
 */
export interface EventDraft {
  streamSemantics: string
  streamName: string
  payload: unknown
}

export const EventDraft = {
  /**
   * Helper function for creating an [[EventDraft]] using the provided
   * semantics, name and payload.
   *
   * @param streamSemantics Semantics of the stream the event should be
   *                        published to
   * @param streamName      Name of the stream the event should be published to
   * @param payload         The event payload.
   * @returns               An event draft that can be passed to the
   *                        [[EventServiceClient]]'s
   *                        [[EventServiceClient.publish | publish]]
   *                        function.
   */
  make: (streamSemantics: string, streamName: string, payload: unknown): EventDraft => ({
    streamSemantics,
    streamName,
    payload,
  }),
}

/**
 * The API client is configurable. This interface defines the properties it
 * requires. In most cases you will not need to configure anything. Refer to
 * [[DefaultClientOpts]] for information about the default client options.
 */
export interface ApiClientOpts {
  /**
   * Endpoints for the different ActyxOS services.
   */
  Endpoints: {
    EventService: {
      BaseUrl: string
      Subscribe: string
      Offsets: string
      Query: string
      Publish: string
    }
  }
}

/**
 * Describes a subscription
 */
export declare type Subscription = {
  streamSemantics?: string | undefined
  streamName?: string | undefined
  source?: string | undefined
}

/**
 * This object provides useful constructors for creating subscription
 * definitions.
 */
export const Subscription = {
  /**
   * This creates a subscription to all events known to the ActyxOS Event
   * Service.
   */
  everything: (): Subscription => ({}),
  /**
   * This creates a subscription to all events with the specified semantics.
   */
  wildcard: (streamSemantics: string): Subscription => ({
    streamSemantics,
  }),
  /**
   * This creates a subscription to all events with the specified semantics and
   * the specified name.
   */
  distributed: (streamSemantics: string, streamName: string): Subscription => ({
    streamSemantics,
    streamName,
  }),
  /**
   * This creates a subscription to all events with the specified semantics and
   * the specified name, and from the specified source.
   */
  local: (streamSemantics: string, streamName: string, source: string): Subscription => ({
    streamSemantics,
    streamName,
    source,
  }),
}

/**
 * This is a callback function type that is used for subscribing to event
 * streams or querying the ActyxOS Event Service (see [[SubscribeOpts]] or
 * [[QueryOpts]]).
 */
export type OnEvent = (event: Event) => void

/**
 * This is a callback function type that is used for notifying users that an
 * error has occured.
 */
export type OnError = (error: string) => void

/**
 * This is a callback function type that is used for notifying users that an
 * operation has completed. This type of callback is used for operations that
 * return results via other means (e.g. [[OnEvent]]).
 */
export type OnDone = () => void

/**
 * This is a callback function type that is used for return the result of an
 * asynchronous operation to the user.
 *
 * @typeParam T the type of the result that will be provided to the callback.
 */
export type OnResult<T> = (result: T) => void

/**
 * Configuration of a subscription to the ActyxOS Event Service. Please refer
 * to the individual properties for more information about what each of them
 * do.
 */
export interface SubscribeOpts {
  /**
   * This property defines the stream subscriptions to be made as part of this
   * subscription. You can subscribe to not only one ActyxOS event stream, but
   * to many streams at the same time. You can do this by providing multiple
   * subscriptions at a time and by using constructors defined in
   * [[Subscription]].
   *
   * Here are a couple of examples.
   *
   *
   * **Create a subscription to everything**
   *
   * ```typescript
   * const subscription = Subscription.everything()
   * ```
   *
   * **Create a subscription for all events with a certain semantic**
   *
   * ```typescript
   * const subscription = Subscription.wildcard('temperatureValue')
   * ```
   *
   * **Create a subscription for all events related to a distributed state**
   *
   * ```typescript
   * const subscription = Subscription.distributed('machineState', 'machine1')
   * ```
   *
   * **Creating multiple subscriptions**
   *
   * ```typescript
   * const subscriptions = [
   *    Subscription.wildcard('temperatureValue')
   *    Subscription.distributed('machineState', 'machine1'),
   * ]
   * ```
   *
   */
  subscriptions: Subscription | Subscription[]
  /**
   * This is a callback that will be called for each event returned by the
   * subscription.
   */
  onEvent: OnEvent
  /**
   * This is a callback that will be called after the subscription has ended.
   * Since subscriptions don't naturally end, this only happens if the
   * subscription is aborted using the provided callback (see
   * [[EventServiceClient.subscribe]]).
   */
  onDone?: OnDone
  /**
   * This is a callback that will be called if an error occurs during the
   * subscription.
   *
   * _Note that a subscription is immediately destroyed if an error occurs._
   */
  onError?: OnError
  /**
   * You can provide an [[OffsetMap]] specifying the lower bound of your
   * subscription. Please refer to the [ActyxOS Event Service documentation](
   * https://developer.actyx.com/docs/os/api/event-service#subscribe-to-event-streams)
   * for more information about how the API handles lower and upper bounds.
   */
  lowerBound?: OffsetMap
}

/**
 * Configuration of a query to the ActyxOS Event Service. Please refer to the
 * individual properties for more information about what each of them do.
 */
export interface QueryOpts {
  /**
   * You can provide an [[OffsetMap]] specifying the lower bound of your query.
   * Please refer to the [ActyxOS Event Service documentation](
   * https://developer.actyx.com/docs/os/api/event-service#subscribe-to-event-streams)
   * for more information about how the API handles lower and upper bounds.
   *
   * _Note: not providing a lower bound means that you will receive all events
   * known to the ActyxOS node's since it was first created._
   */
  lowerBound?: OffsetMap

  /**
   * You must provide an [[OffsetMap]] specifying the upper bound of your
   * subscription. Please refer to the [ActyxOS Event Service documentation](
   * https://developer.actyx.com/docs/os/api/event-service#subscribe-to-event-streams)
   * for more information about how the API handles lower and upper bounds.
   */
  upperBound: OffsetMap

  /**
   * This property defines the stream subscriptions to be made as part of this
   * query. You can query not only one ActyxOS event stream, but many streams
   * at the same time. You can do this by providing multiple subscriptions at a
   * time and by using constructors defined in [[Subscription]].
   *
   * Here are a couple of examples.
   *
   *
   * **Create a subscription to everything**
   *
   * ```typescript
   * const subscription = Subscription.everything()
   * ```
   *
   * **Create a subscription for all events with a certain semantic**
   *
   * ```typescript
   * const subscription = Subscription.wildcard('temperatureValue')
   * ```
   *
   * **Create a subscription for all events related to a distributed state**
   *
   * ```typescript
   * const subscription = Subscription.distributed('machineState', 'machine1')
   * ```
   *
   * **Creating multiple subscriptions**
   *
   * ```typescript
   * const subscriptions = [
   *    Subscription.wildcard('temperatureValue')
   *    Subscription.distributed('machineState', 'machine1'),
   * ]
   * ```
   *
   */
  subscriptions: Subscription | Subscription[]

  /**
   * The order in which you want the query to return results. Please refer to
   * the ActyxOS [Event Service API documentation](
   * https://developer.actyx.com/docs/os/api/event-service#query-event-streams)
   * for more details.
   */
  ordering: Ordering
  /**
   * This is a callback that will be called for each event returned by the
   * query.
   */
  onEvent: OnEvent

  /**
   * This is a callback that will be called after the query has ended.
   */
  onDone?: OnDone

  /**
   * This is a callback that will be called if an error occurs during the
   * query.
   *
   * _Note that a query is immediately destroyed if an error occurs._
   */
  onError?: OnError
}

/**
 * Configuration of a publishing request to the ActyxOS Event Service. Please
 * refer to the individual properties for more information about what each of
 * them do.
 */
export interface PublishOpts {
  eventDrafts: EventDraft | EventDraft[]
  /**
   * This is a callback that will be called after the event(s) have been
   * successfully published.
   */
  onDone?: OnDone

  /**
   * This is a callback that will be called if any errors occur during the
   * publication process.
   */
  onError?: OnError
}

/**
 * Configuration of a request for offsets to the ActyxOS Event Service. Please
 * refer to the individual properties for more information about what each of
 * them do.
 */
export interface OffsetsOpts {
  /**
   * This is a callback that will be called with the offsets once they have been
   * received from the Event Service.
   */
  onOffsets: OnResult<OffsetMap>

  /**
   * This is a callback that will be called if any errors occur whilst getting
   * offsets from the Event Service.
   */
  onError?: OnError
}

/**
 * This interface specifies the functionality that this SDK offers for
 * interacting with the ActyxOS [Event Service](
 * https://developer.actyx.com/docs/os/api/event-service)
 */
export interface EventServiceClient {
  /**
   * This function allows you to perform a subscription to the ActyxOS Event
   * Service. Because subscriptions never end, this function only returns if
   * the subscription is aborted by you using the returned callback, or if an
   * error occurs. Please refer to the [[SubscribeOpts]] docs for more details
   * about how to _configure_ your subscription.
   * 
   * **Example usage**
   * 
   * ```typescript
   * const stopSubscription = client.eventService.subscribe({
   *   // Specify a lower bound of offsets; we will not get events below that
   *   // offset
   *   lowerBound: offsets,
   * 
   *   // Define a subscription to all events in all event streams
   *   subscriptions: Subscription.everything(),
   * 
   *   // Provide a callback that will be called for every event that is
   *   // returned by the subscription
   *   onEvent: event => {
   *     console.log('Event:')
   *     console.log(JSON.stringify(event, null, 2))
   *   },
   * 
   *   // This callback will be called if you manually abort the subscription
   *   // using the function returned by this function
   *   onDone: () => {
   *     console.log(`Subscription done!`)
   *   },
   * 
   *   // This callback will be called if any error occurs during the execution
   *   // of the subscription
   *   onError: error => {
   *     console.error(`error during subscription: ${error}`)
   *   },
     })
   * ```
   * 
   * If you want to stop the subscription at some point, you could do so by
   * using the function returned by this function:
   * 
   * ```typescript
   * const stopSubscription = client.eventService.subscribe(opts)
   * 
   * // Later
   * stopSubscription()
   * ```
   * 
   * @param opts Options for the subscription (see [[SubscribeOpts]]).
   * @returns    Callback for aborting the subscription. Calling the returned
   *             function will stop the subscription. If you have provided an
   *             [[OnDone]] callback in the [[SubscribeOpts]], it will be called
   *             when the subscription ends.
   */
  subscribe: (opts: SubscribeOpts) => () => void

  /**
   * This function allows you to query the ActyxOS Event Service. As opposed to
   * subscriptions, a query will always end. Please refer to the [[QueryOpts]]
   * docs for more details about how to _configure_ your query.
   *
   * **Example usage**
   *
   * ```typescript
   * client.eventService.query({
   *
   *   // Define an upper bound for the query
   *   upperBound: toOffsets,
   *
   *   // Order the events using their lamport timestamp
   *   ordering: Ordering.Lamport,
   *
   *   // Specify the streams you would like to query by their semantics and
   *   // name
   *   subscriptions: Subscription.distributed('machineState', 'machine1'),
   *
   *  // This is the callback that will be called for every event that the query
   *  // returns
   *   onEvent: event => {
   *     console.log(`query returned event: ${JSON.stringify(event)}`)
   *   },
   *
   *   // This callback will be called when the query end, i.e. when the last
   *   // event that it returned has been passed to the [[OnEvent]] callback
   *   onDone: () => {
   *     console.log('query completed')
   *   },
   *
   *   // This callback will be called if there is any error in the executing
   *   // the query
   *   onError: error => {
   *     console.error(`error querying: ${error}`)
   *   },
   * })
   * ```
   *
   * @param opts Options for the query (see [[QueryOpts]]).
   */
  query: (opts: QueryOpts) => void

  /**
   * This function allows you to publish events using the ActyxOS Event Service.
   * Please refer to the [[PublishOpts]] docs for more details about how to
   * _configure_ your call to this function.
   *
   * **Example usage**
   *
   * ```typescript
   * client.eventService.publish({
   *
   *   // Pass in one or more event drafts that you want to publish
   *   eventDrafts: EventDraft.make('testSemantics', 'testName', { foo: 'bar' }),
   *
   *   // This is callback that will be called when the provided event draft(s)
   *   // have been published to the Event Service
   *   onDone: () => {
   *     console.log(`Published`)
   *   },
   *
   *   // This function will be called if an error occurs during the publishing
   *   // operation
   *   onError: error => {
   *     console.error(`error publishing: ${error}`)
   *   },
   * })
   * ```
   *
   * @param opts Options for the publish (see [[PublishOpts]]).
   */
  publish: (opts: PublishOpts) => void

  /**
   * This function allows you to get all known offsets from the ActyxOS Event
   * Service. Please refer to the [[OffsetsOpts]] docs for more details about how
   * to _configure_ your call to this function.
   *
   * **Example usage**
   *
   * ```typescript
   * client.eventService.offsets({
   *
   *   // This callback will be called when the API has returned the offsets
   *   onOffsets: offsets => {
   *     console.log(`Got offsets: ${JSON.stringify(offsets, null, 2)}`)
   *   },
   *
   *   // This callback will be called if an error occurs whilst trying to get
   *   // the offsets
   *   onError: error => {
   *     console.error(`error getting offsets: ${error}`)
   *   },
   * })
   * ```
   *
   * @param opts Options for the publish (see [[PublishOpts]]).
   */
  offsets: (opts: OffsetsOpts) => void
}

/**
 * Definition of the API client. The client currently offers access to the
 * ActyxOS Event Service via it's [[eventService]] property.
 */
export interface ApiClient {
  /**
   * Access to the ActyxOS Event Service using the
   * [[EventServiceClient.offsets | offsets]],
   * [[EventServiceClient.query | query]],
   * [[EventServiceClient.subscribe | subscribe]] and
   * [[EventServiceClient.publish | publish]] functions.
   */
  eventService: EventServiceClient
}
