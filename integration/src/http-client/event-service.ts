import { Decoder } from 'io-ts'
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
  AxNodeService,
  OffsetsResponse,
  OnData,
  PublishResponse,
  QueryResponse,
  SubscribeMonotonicResponse,
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

export const handleStreamResponse = async <T>(
  decoder: Decoder<unknown, T>,
  onData: OnData<T>,
  stream: NodeJS.ReadableStream,
  onCancel?: () => void,
): Promise<void> => {
  let canceled = false
  const cb = (data: T) =>
    onData(data, () => {
      canceled = true
      onCancel && onCancel()
    })
  const lines$ = stream.pipe(mkLinesSplitter())
  const parse = mkLineParser(decoder)
  for await (const line of lines$) {
    if (canceled) {
      break
    }
    cb(parse(line))
  }
}

export const mkNodeIdService = (httpClient: AxHttpClient): AxNodeService => ({
  nodeId: () => httpClient.fetch(API_V2_PATH + NODE_ID_SEG).then((x) => x.text()),
})

export const mkEventService = (httpClient: AxHttpClient): AxEventService => ({
  offsets: () =>
    httpClient
      .get(mkEventsPath(OFFSETS_SEG))
      .then((x) => x.json())
      .then((x) => decodeOrThrow(OffsetsResponse)(x)),
  publish: (request) =>
    httpClient
      .post(mkEventsPath(PUBLISH_SEG), request)
      .then((x) => x.json())
      .then((x) => decodeOrThrow(PublishResponse)(x)),
  query: async (request, onData) => {
    const res = await httpClient.post(mkEventsPath(QUERY_SEG), request, true)
    await handleStreamResponse(QueryResponse, onData, res.body)
  },
  subscribe: async (request, onData) => {
    const res = await httpClient.post(mkEventsPath(SUBSCRIBE_SEG), request, true)
    await handleStreamResponse(SubscribeResponse, onData, res.body)
  },
  subscribeMonotonic: async (request, onData) => {
    const res = await httpClient.post(mkEventsPath(SUBSCRIBE_MONOTONIC_SEG), request, true)

    await handleStreamResponse(SubscribeMonotonicResponse, onData, res.body)
  },
})
