// TODO: convert to io-ts
export type Event = Readonly<{
  type: string
  lamport: number
  stream: string
  offset: number
  timestamp: number
  tags: ReadonlyArray<string>
  payload: Readonly<{ value: string }>
}>

export type EventDelivered = Event &
  Readonly<{
    caughtUp: boolean
  }>

export type ErrorResponse = Readonly<{
  code: string
  message: string
}>

export type NodeIdResponse = Readonly<{
  nodeId: string
}>

export type OffsetsResponse = Readonly<Record<string, number>>

export type PublishResponse = Readonly<{
  data: ReadonlyArray<{
    lamport: number
    stream: string
    offset: number
    timestamp: number
  }>
}>
