import { AxiosResponse } from 'axios'
import { NodeIdResponse, PublishResponse } from './event-service-types'
import { httpClient } from './httpClient'

export const randomString = (): string =>
  Math.random()
    .toString(36)
    .replace(/[^a-z]+/g, '')
    .substr(0, 5)

const publishEventWithTag = (tags: ReadonlyArray<string>) => (
  value: string,
): Promise<AxiosResponse<PublishResponse>> =>
  httpClient.post('publish', {
    data: [{ tags, payload: { value } }],
  })

export const publishEvent = publishEventWithTag(['integration', 'test:1'])

export const getNodeId = (): Promise<string> =>
  httpClient.get<NodeIdResponse>('node_id').then((x) => x.data.nodeId)

export const mkStreamId = (nodeId: string): string => `${nodeId}-0`
