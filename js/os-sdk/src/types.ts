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
 * A stream of events that can be consumed with a `for-await-loop` and that can
 * also be cancelled externally. The second part is needed when cancellation may
 * occur during the `await` phase of the loop, in which case calling `cancel` is
 * the only way to correctly release all stream resources.
 */
export interface EventStream extends AsyncIterable<Event> {
  /**
   * Cancel the stream and release all related resources.
   */
  cancel: () => void
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
    ConsoleService: {
      BaseUrl: string
      Logs: string
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
export type SubscribeOpts = {
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

export type SubscribeStreamOpts = Omit<SubscribeOpts, 'onEvent' | 'onDone' | 'onError'>

/**
 * Configuration of a query to the ActyxOS Event Service. Please refer to the
 * individual properties for more information about what each of them do.
 */
export type QueryOpts = {
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

export type QueryStreamOpts = Omit<QueryOpts, 'onEvent' | 'onDone' | 'onError'>

/**
 * Configuration of a publishing request to the ActyxOS Event Service. Please
 * refer to the individual properties for more information about what each of
 * them do.
 */
export type PublishOpts = {
  /**
   * The event drafts to be published (and thus turned into events).
   */
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
 * Configuration of a publishing request to the ActyxOS Event Service. Please
 * refer to the individual properties for more information about what each of
 * them do.
 */
export type PublishPromiseOpts = Omit<PublishOpts, 'onDone' | 'onError'>

/**
 * Configuration of a request for offsets to the ActyxOS Event Service. Please
 * refer to the individual properties for more information about what each of
 * them do.
 */
export type OffsetsOpts = {
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
 * https://developer.actyx.com/docs/os/api/event-service).
 */
export interface EventServiceClient {
  /**
   * This function allows you to perform a subscription to the ActyxOS Event
   * Service. Please refer to the [[SubscribeOpts]] docs for more details
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
   * })
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
   * This function allows you to perform a subscription to the ActyxOS Event
   * Service. Please refer to the [[SubscribeStreamOpts]] docs for more details
   * about how to _configure_ your subscription.
   *
   * **Example usage**
   *
   * ```typescript
   * const subscription = await client.eventService.subscribeStream({
   *   // Specify a lower bound of offsets; we will not get events below that
   *   // offset
   *   lowerBound: offsets,
   *
   *   // Define a subscription to all events in all event streams
   *   subscriptions: Subscription.everything(),
   * })
   *
   * for await (const event of subscription) {
   *   console.log('Event:')
   *   console.log(JSON.stringify(event, null, 2))
   *   // in case you don’t need further events:
   *   if (isDone(event)) {
   *     break
   *     // this will also terminate the subscription and release resources
   *   }
   * }
   * ```
   *
   * If you want to stop the subscription based on an external event, you can do so by
   * using the `.cancel()` method of the returned subscription:
   *
   * ```typescript
   * const subscription = await client.eventService.subscribeStream(opts)
   *
   * // Later
   * subscription.cancel()
   * ```
   *
   * This is not needed when using `for await (...)` and breaking or finishing the loop,
   * in which case the loop itself will handle stream cancellation.
   *
   * @param opts Options for the subscription (see [[SubscribeStreamOpts]]).
   * @returns    An async iterable that also contains a callback for terminating the
   *             stream if you need to do that unrelated to the for-await-loop.
   */
  subscribeStream: (opts: SubscribeStreamOpts) => Promise<EventStream>

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
   * This function allows you to query the ActyxOS Event Service. As opposed to
   * subscriptions, a query will always end. Please refer to the [[QueryOpts]]
   * docs for more details about how to _configure_ your query.
   *
   * **Example usage**
   *
   * ```typescript
   * const subscription = await client.eventService.queryStream({
   *   // Specify a lower bound of offsets; we will not get events below that
   *   // offset
   *   lowerBound: offsets,
   *
   *   // Define an upper bound for the query
   *   upperBound: toOffsets,
   *
   *   // Order the events using their lamport timestamp
   *   ordering: Ordering.Lamport,
   *
   *   // Define a subscription to all events in all event streams
   *   subscriptions: Subscription.everything(),
   * })
   *
   * for await (const event of subscription) {
   *   console.log('Event:')
   *   console.log(JSON.stringify(event, null, 2))
   *   // in case you don’t need further events:
   *   if (isDone(event)) {
   *     break
   *     // this will also terminate the subscription and release resources
   *   }
   * }
   * ```
   *
   * If you want to stop the subscription based on an external event, you can do so by
   * using the `.cancel()` method of the returned subscription:
   *
   * ```typescript
   * const subscription = await client.eventService.queryStream(opts)
   *
   * // Later
   * subscription.cancel()
   * ```
   *
   * This is not needed when using `for await (...)` and breaking or finishing the loop,
   * in which case the loop itself will handle stream cancellation.
   *
   * @param opts Options for the query (see [[QueryStreamOpts]]).
   * @returns    An async iterable that also contains a callback for terminating the
   *             stream if you need to do that unrelated to the for-await-loop.
   */
  queryStream: (opts: QueryStreamOpts) => Promise<EventStream>

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
   * This function allows you to publish events using the ActyxOS Event Service.
   * Please refer to the [[PublishOpts]] docs for more details about how to
   * _configure_ your call to this function.
   *
   * **Example usage**
   *
   * ```typescript
   * await client.eventService.publishPromise({
   *   // Pass in one or more event drafts that you want to publish
   *   eventDrafts: EventDraft.make('testSemantics', 'testName', { foo: 'bar' }),
   * })
   * ```
   *
   * @param opts Options for the publish (see [[PublishOpts]]).
   */
  publishPromise: (opts: PublishPromiseOpts) => Promise<void>

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

  /**
   * This function allows you to get all known offsets from the ActyxOS Event
   * Service. Please refer to the [[OffsetsOpts]] docs for more details about how
   * to _configure_ your call to this function.
   *
   * **Example usage**
   *
   * ```typescript
   * const offsets = await client.eventService.offsetsPromise()
   * // now use them e.g. for a .queryStream() as upperBound
   * ```
   *
   * @returns A promise that will be fulfilled with the currently known offsets.
   */
  offsetsPromise: () => Promise<OffsetMap>
}

export enum LogSeverity {
  DEBUG,
  INFO,
  WARN,
  ERROR,
}

/**
 * This data type specifies the content of a log entry that must/can be provided
 * by your application. This is not exactly equivalent to the actual log entry
 * stored by ActyxOS since the Console Service automatically adds additional
 * data points such as the node's ID or display name. The Console Service also
 * adds the timestamp unless you have specifically provided it.
 *
 * Have a look at the Console Service's
 * [API documentation](/docs/os/api/console-service#structured-vs-unstructured-logs)
 * for more details, including how ActyxOS stores log entries internally.
 *
 * The most basic [[LogEntryDraft]] would look as follows:
 *
 * ```typescript
 * const minimalEntry: LogEntryDraft = {
 *  logName: 'myLogger',
 *  severity: LogSeverity.INFO,
 *  producer: {
 *   name: 'myapp',
 *   version: '1.0.0',
 * },
 * message: 'my log message',
 * }
 * ```
 */
export type LogEntryDraft = {
  /**
   * Timestamp of the log entry.
   *
   * _Note: ActyxOS automatically adds this, so you should almost never have to
   * set this yourself._
   */
  timestamp?: Date
  /**
   * Name of the log you want to post the entry to.
   */
  logName: string
  /**
   * Severity of the log entry (debug, warn, info or error).
   */
  severity: LogSeverity
  /**
   * Details about who produced the log message. This should be your app's name
   * and version number.
   */
  producer: {
    name: string
    version: string
  }
  /**
   * Any labels you would like to add to the log entry.
   *
   * _Note that labels are always of type `string: string`._
   */
  labels?: {
    [key: string]: string
  }
  /**
   * The actual log message.
   */
  message: string
  /**
   * Additional data you would like to add to the log entry. _Note: this is data
   * that complements the log message (e.g. additional parsing errors)._
   */
  additionalData?: unknown
}

/**
 * Configuration of a request to post a log entry to the ActyxOS Console
 * Service. Please refer to the individual properties for more information
 * about what each of them do.
 */
export type LogOpts = {
  /**
   * This property defines the draft log entry you want to post to the Console
   * Service. The log entry has a number of required and a number of optional
   * properties. Please refer to [[LogEntryDraft]] for more information and a
   * couple of examples.
   */
  entry: LogEntryDraft

  /**
   * This property allows you to provide a callback that will be called when the
   * log entry has been successfully posted.
   *
   * _Note: in most cases you can simply fire-and-forget and thus don't need to
   * provide this callback._
   */
  onLogged?: OnResult<void>

  /**
   * This property allows you to provide a callback that will be called if an
   * error occurs whilst the SDK tried to post the log entry.
   *
   * _Note: this should not happen unless there are problems with the ActyxOS
   * node so if anything you might throw an exception here._
   */
  onError?: OnError
}

/**
 * A simple logger you can use in your app. It provides four functions, one for
 * each of the available [[LogSeverity]] levels. Check out
 * [[ConsoleServiceClient.SimpleLogger]] if you want to see some usage examples.
 */
export interface SimpleLogger {
  /**
   * Log a debug message and optionally add additional data.
   *
   * @param msg            Message to be logged
   * @param additionalData Additional data you want to attach to the log entry
   */
  debug: (msg: string, additionalData?: unknown) => void
  /**
   * Log an info message and optionally add additional data.
   *
   * @param msg            Message to be logged
   * @param additionalData Additional data you want to attach to the log entry
   */
  info: (msg: string, additionalData?: unknown) => void
  /**
   * Log a warning message and optionally add additional data.
   *
   * @param msg            Message to be logged
   * @param additionalData Additional data you want to attach to the log entry
   */
  warn: (msg: string, additionalData?: unknown) => void
  /**
   * Log an error message and optionally add additional data.
   *
   * @param msg            Message to be logged
   * @param additionalData Additional data you want to attach to the log entry
   */
  error: (msg: string, additionalData?: unknown) => void
}

/**
 * Configuration parameters for creating a [[SimpleLogger]]. You must specify
 * the producer name and version (e.g. `myapp` and `1.0.0`) and a name for the
 * log you want to post to (e.g. `myLogger`). You may also add a default error
 * callback that will be called whenever an error occurs whilst trying to post
 * a log entry.
 */
export type SimpleLoggerOpts = {
  producerName: string
  producerVersion: string
  logName: string
  onError?: (error: string) => void
}

/**
 * This interface specifies the functionality that this SDK offers for
 * interacting with the ActyxOS [Console Service](
 * https://developer.actyx.com/docs/os/api/console-service).
 */
export interface ConsoleServiceClient {
  /**
   * This function allows you to log to the ActyxOS Console Service. You can
   * pass in either a [[LogEntryDraft]] object or a [[LogOpts]] object. You must
   * use the [[LogOpts]] object if you want to provide callback for when the
   * log entry has been successfully posted or if an error occurs.
   *
   * **Example usage**
   *
   * ```typescript
   * import { Client } from '@actyx/os-sdk'
   *
   * const ActyxOS = Client()
   *
   * ActyxOS.consoleService.log({
   *   entry: {
   *     logName: 'myCustomLogger',
   *     message: 'this is a WARNING message',
   *     severity: LogSeverity.WARN,
   *     producer: {
   *       name: 'com.example.app1',
   *       version: '1.0.0'
   *     },
   *     additionalData: {
   *       foo: 'bar',
   *       bar: {
   *         foo: true,
   *       }
   *     },
   *     labels: {
   *       'com.example.app1.auth.username': 'john.doe',
   *       'com.example.app1.model.events': '10000',
   *     }
   *   },
   *   // Callback on successful logging
   *   onLogged: () => {
   *     // Do something
   *   },
   *   // Callback on error logging
   *   onError: err => {
   *     console.error(`error logging: ${err}`)
   *   }
   * })
   * ```
   *
   * @param opts Either a [[LogEntryDraft]] or a [[LogOpts]] object
   */
  log: (opts: LogOpts | LogEntryDraft) => void

  /**
   * This function allows you to log a [[LogEntryDraft]] to the ActyxOS Console Service.
   *
   * **Example usage**
   *
   * ```typescript
   * import { Client } from '@actyx/os-sdk'
   *
   * const ActyxOS = Client()
   *
   * await ActyxOS.consoleService.logPromise({
   *   logName: 'myCustomLogger',
   *   message: 'this is a WARNING message',
   *   severity: LogSeverity.WARN,
   *   producer: {
   *     name: 'com.example.app1',
   *     version: '1.0.0'
   *   },
   *   additionalData: {
   *     foo: 'bar',
   *     bar: {
   *       foo: true,
   *     }
   *   },
   *   labels: {
   *     'com.example.app1.auth.username': 'john.doe',
   *     'com.example.app1.model.events': '10000',
   *   }
   * })
   * ```
   *
   * @param opts a [[LogEntryDraft]]
   */
  logPromise: (entry: LogEntryDraft) => Promise<void>

  /**
   * Create a simple logger with `debug`, `warn`, `info` and `error` functions
   * to easily post log entries.
   *
   * **Example usage**
   * ```
   * import { Client } from '@actyx/os-sdk'
   *
   * const ActyxOS = Client()
   *
   * const logger: SimpleLogger = ActyxOS.consoleService.SimpleLogger({
   *   logName: 'myLogger',
   *   producerName: 'com.example.app1',
   *   producerVersion: '1.0.0'
   * })
   *
   * logger.debug('this is a DEBUG message')
   * logger.warn('this is a WARNING message')
   * logger.info('this is an INFO message')
   * logger.error('this is an ERROR message')
   *
   * logger.debug('This is a message with additional data', {foo: 'bar'})
   * ```
   *
   * Please refer to [[SimpleLoggerOpts]] for more details about how to
   * configure this simple logger.
   *
   */
  SimpleLogger: (opts: SimpleLoggerOpts) => SimpleLogger
}

/**
 * Definition of the API client. The client currently offers access to the
 * ActyxOS Event Service via it's [[eventService]] property and to the
 * ActyxOS Console Service via it's [[consoleService]] property.
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
  /**
   * Access to the ActyxOS Console Service using the
   * [[ConsoleServiceClient.SimpleLogger | SimpleLogger]] and
   * [[ConsoleServiceClient.log | log]] functions.
   */
  consoleService: ConsoleServiceClient
}
