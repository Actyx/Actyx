import { Decoder } from 'io-ts'
import { Response } from 'node-fetch'
import { AxHttpClient } from './ax-http-client'
import {
  API_V2_PATH,
  EVENTS_PATH,
  NODE_ID_SEG,
  OFFSETS_SEG,
  PUBLISH_SEG,
  QUERY_SEG,
  SUBSCRIBE_MONOTONIC_SEG,
  SUBSCRIBE_SEG,
} from './const'
import { decodeOrThrow } from './decode-or-throw'
import { mkLinesSplitter } from './line-splitter'
import {
  AxEventService,
  NodeIdResponse,
  OffsetsResponse,
  PublishRequest,
  PublishResponse,
  QueryRequest,
  QueryResponse,
  SubscribeMonotonicRequest,
  SubscribeMonotonicResponse,
  SubscribeRequest,
  SubscribeResponse,
} from './types'

export const mkEventsPath = (segment: string): string => API_V2_PATH + EVENTS_PATH + `${segment}`

const mkLineParser = <T>(decoder: Decoder<unknown, T>): ((data: string) => T) => {
  const dec = decodeOrThrow(decoder)
  return (line) => {
    let data: unknown
    try {
      data = JSON.parse(line)
    } catch (err) {
      throw `unable to parse line '${line}' as JSON: ${err}`
    }

    return dec(data)
  }
}

const handleStreamResponse = async <T>(
  decoder: Decoder<unknown, T>,
  onData: (data: T) => void,
  response: Response,
): Promise<void> => {
  const lines$ = response.body.pipe(mkLinesSplitter())
  const parse = mkLineParser(decoder)
  for await (const line of lines$) {
    onData(parse(line))
  }
}

export const mkEventService = (httpClient: AxHttpClient): AxEventService => {
  return {
    nodeId: () =>
      httpClient
        .get(mkEventsPath(NODE_ID_SEG))
        .then((x) => x.json())
        .then((x) => decodeOrThrow(NodeIdResponse)(x)),
    offsets: () =>
      httpClient
        .get(mkEventsPath(OFFSETS_SEG))
        .then((x) => x.json())
        .then((x) => decodeOrThrow(OffsetsResponse)(x)),
    publish: (request: PublishRequest) =>
      httpClient
        .post(mkEventsPath(PUBLISH_SEG), request)
        .then((x) => x.json())
        .then((x) => decodeOrThrow(PublishResponse)(x)),
    query: async (request: QueryRequest, onData: (response: QueryResponse) => void) => {
      const res = await httpClient.post(mkEventsPath(QUERY_SEG), request, true)
      await handleStreamResponse(QueryResponse, onData, res)
    },
    subscribe: async (request: SubscribeRequest, onData: (response: SubscribeResponse) => void) => {
      const res = await httpClient.post(mkEventsPath(SUBSCRIBE_SEG), request, true)
      await handleStreamResponse(SubscribeResponse, onData, res)
    },
    subscribeMonotonic: async (
      request: SubscribeMonotonicRequest,
      onData: (response: SubscribeMonotonicResponse) => void,
    ) => {
      const res = await httpClient.post(mkEventsPath(SUBSCRIBE_MONOTONIC_SEG), request, true)
      await handleStreamResponse(SubscribeMonotonicResponse, onData, res)
    },
  }
}
