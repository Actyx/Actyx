import * as t from 'io-ts'

export const OffsetMap = t.record(t.string, t.number)
export type OffsetMap = t.TypeOf<typeof OffsetMap>

export const OffsetsResponse = t.type({
  present: OffsetMap,
  toReplicate: t.record(t.string, t.number),
})
export type OffsetsResponse = t.TypeOf<typeof OffsetsResponse>

export const PublishResponseKey = t.type({
  lamport: t.number,
  stream: t.string,
  offset: t.number,
  timestamp: t.number,
})
export type PublishResponseKey = t.TypeOf<typeof PublishResponseKey>
export const PublishResponse = t.type({
  data: t.readonlyArray(PublishResponseKey),
})
export type PublishResponse = t.TypeOf<typeof PublishResponse>

// Streams
export const EventResponse = t.type({
  type: t.literal('event'),
  lamport: t.number,
  stream: t.string,
  offset: t.number,
  timestamp: t.number,
  tags: t.readonlyArray(t.string),
  payload: t.unknown,
})
export type EventResponse = t.TypeOf<typeof EventResponse>

export const QueryResponse = EventResponse
export type QueryResponse = t.TypeOf<typeof QueryResponse>

export const SubscribeResponse = EventResponse
export type SubscribeResponse = t.TypeOf<typeof SubscribeResponse>

// SubscribeMonotonic
export const Compression = t.union([t.literal('none'), t.literal('deflate')])
export type Compression = t.TypeOf<typeof Compression>

export const SubscribeMonotonicState = t.type({
  type: t.literal('state'),
  snapshot: t.type({
    compression: Compression,
    // base64?
    data: t.string,
  }),
})
export type SubscribeMonotonicState = t.TypeOf<typeof SubscribeMonotonicState>

export const SubscribeMonotonicEvent = t.intersection([
  EventResponse,
  t.type({
    caughtUp: t.boolean,
  }),
])
export type SubscribeMonotonicEvent = t.TypeOf<typeof SubscribeMonotonicEvent>

export const EventKey = t.type({
  lamport: t.number,
  stream: t.string,
  offset: t.number,
})
export type EventKey = t.TypeOf<typeof EventKey>

export const SubscribeMonotonicTimeTravel = t.type({
  type: t.literal('timeTravel'),
  newStart: EventKey,
})
export type SubscribeMonotonicTimeTravel = t.TypeOf<typeof SubscribeMonotonicTimeTravel>

export const SubscribeMonotonicResponse = t.union([
  SubscribeMonotonicState,
  SubscribeMonotonicEvent,
  SubscribeMonotonicTimeTravel,
])
export type SubscribeMonotonicResponse = t.TypeOf<typeof SubscribeMonotonicResponse>

export type PublishEvent = {
  tags: ReadonlyArray<string>
  payload: unknown
}
export type PublishRequest = {
  data: ReadonlyArray<PublishEvent>
}
export enum Order {
  /// Events are sorted by ascending Lamport timestamp and stream ID, which defines a
  /// total order.
  Asc = 'asc',
  /// Events are sorted by descending Lamport timestamp and descending stream ID,
  /// which is the exact reverse of the `Asc` ordering.
  Desc = 'desc',
  /// Events are sorted within each stream by ascending Lamport timestamp, with events
  /// from different streams interleaved in an undefined order.
  StreamAsc = 'streamAsc',
}
export type QueryRequest = {
  lowerBound?: OffsetMap
  upperBound: OffsetMap
  query: string
  order: Order
}
export type SubscribeRequest = {
  lowerBound?: OffsetMap
  query: string
}
export type SubscribeMonotonicRequestStartFrom =
  | { lowerBound: OffsetMap }
  | { snapshot: ReadonlyArray<Compression> }

export type SubscribeMonotonicRequest = {
  session: string
  query: string
} & SubscribeMonotonicRequestStartFrom

export type AxNodeService = Readonly<{
  nodeId: () => Promise<string>
}>

export type AxEventService = Readonly<{
  offsets: () => Promise<OffsetsResponse>
  publish: (request: PublishRequest) => Promise<PublishResponse>
  /**
   * Resolves when stream completes. Use `catch` for error handling
   */
  query: (request: QueryRequest, onData: (response: QueryResponse) => void) => Promise<void>
  /**
   * The returned Promise rejects if there is an error, otherwise never resolves.
   */
  subscribe: (
    request: SubscribeRequest,
    onData: (response: SubscribeResponse) => void,
  ) => Promise<void>
  /**
   * Resolves when stream completes. Use `catch` for error handling
   */
  subscribeMonotonic: (
    request: SubscribeMonotonicRequest,
    onData: (response: SubscribeMonotonicResponse) => void,
  ) => Promise<void>
}>
export const ErrorCode = t.union([
  t.literal('ERR_BAD_REQUEST'),
  t.literal('ERR_NOT_FOUND'),
  t.literal('ERR_MISSING_AUTH_HEADER'),
  t.literal('ERR_MALFORMED_REQUEST_SYNTAX'),
  t.literal('ERR_METHOD_NOT_ALLOWED'),
  t.literal('ERR_NOT_ACCEPTABLE'),
  t.literal('ERR_TOKEN_INVALID'),
  t.literal('ERR_UNSUPPORTED_AUTH_TYPE'),
])
export type ErrorCode = t.TypeOf<typeof ErrorCode>

export const ErrorResponse = t.type({
  code: ErrorCode,
  message: t.string,
})
export type ErrorResponse = t.TypeOf<typeof ErrorResponse>
