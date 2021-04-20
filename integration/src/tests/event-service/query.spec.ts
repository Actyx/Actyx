import { httpClient, REQUEST_OPTIONS_QUERY } from '../../httpClient'
import { getNodeId, mkStreamId, publishEvent, randomString } from '../../util'
import { getEventsInStreamAfterMs } from '../../stream'
import { ErrorResponse, Event } from '../../event-service-types'
import { AxiosError } from 'axios'

const QUERY_TIMEOUT = 500

describe('event service', () => {
  describe('query event streams', () => {
    it('should return error if stream is not valid', async () => {
      await httpClient
        .post('query', {
          upperBound: {
            'not-existing.0': 100,
          },
          where: "'integration' & 'test:1'",
          order: 'desc',
        })
        .catch((error: AxiosError<ErrorResponse>) =>
          expect(error).toMatchErrorMalformedRequestSytantax(),
        )
    })

    it('should return events in ascending order', async () => {
      const nodeId = await getNodeId()
      await publishEvent(randomString())
      await publishEvent(randomString())

      const streamId = mkStreamId(nodeId)
      const requestBody: string = JSON.stringify({
        lowerBound: { [streamId]: 0 },
        upperBound: { [streamId]: Number.MAX_SAFE_INTEGER },
        where: "'integration'",
        order: 'asc',
      })
      await getEventsInStreamAfterMs<Event>(REQUEST_OPTIONS_QUERY, QUERY_TIMEOUT, requestBody).then(
        (events) => {
          const penultimateEvent = events.slice(-2)[0]
          const penultimateEventIdx = events.findIndex((x) => x.offset === penultimateEvent.offset)
          const lastEvent = events.slice(-1)[0]
          const latEventIdx = events.findIndex((x) => x.offset === lastEvent.offset)

          expect(lastEvent.offset - penultimateEvent.offset).toBe(1)
          expect(penultimateEventIdx).toBe(events.length - 2)
          expect(latEventIdx).toBe(events.length - 1)

          expect(lastEvent).toMatchObject({
            type: 'event',
            lamport: expect.any(Number),
            stream: streamId,
            offset: expect.any(Number),
            timestamp: expect.any(Number),
            tags: ['integration', 'test:1'],
            payload: { value: expect.any(String) },
          })
        },
      )
    })

    it('should return events in descending order', async () => {
      const nodeId = await getNodeId()
      publishEvent(randomString())
      publishEvent(randomString())
      const streamId = mkStreamId(nodeId)
      const requestBody = JSON.stringify({
        lowerBound: { [streamId]: 0 },
        upperBound: { [streamId]: Number.MAX_SAFE_INTEGER },
        where: "'integration'",
        order: 'desc',
      })
      await getEventsInStreamAfterMs<Event>(REQUEST_OPTIONS_QUERY, QUERY_TIMEOUT, requestBody).then(
        (events) => {
          const [firstEvent, secondEvent] = events
          const firstEventIdx = events.findIndex((x) => x.offset === firstEvent.offset)
          const secondEventIdx = events.findIndex((x) => x.offset === secondEvent.offset)

          expect(firstEvent.offset - secondEvent.offset).toBe(1)
          expect(firstEventIdx).toBe(0)
          expect(secondEventIdx).toBe(1)
        },
      )
    })

    it('should return no events if upperBound is not in range', async () => {
      const nodeId = await getNodeId()
      const streamId = mkStreamId(nodeId)
      const requestBody = JSON.stringify({
        lowerBound: { [streamId]: Number.MAX_SAFE_INTEGER },
        upperBound: { [streamId]: 0 },
        where: "'integration'",
        order: 'asc',
      })
      await getEventsInStreamAfterMs<Event>(REQUEST_OPTIONS_QUERY, QUERY_TIMEOUT, requestBody).then(
        (events) => {
          expect(events).toHaveLength(0)
        },
      )
    })

    it('should return events within lower and upper bound only', async () => {
      const nodeId = await getNodeId()
      const offset1 = await publishEvent(randomString()).then((x) => x.data.data[0].offset)
      const offset2 = await publishEvent(randomString()).then((x) => x.data.data[0].offset)
      const offset3 = await publishEvent(randomString()).then((x) => x.data.data[0].offset)

      const streamId = mkStreamId(nodeId)
      const requestBody = JSON.stringify({
        lowerBound: { [streamId]: offset1 },
        upperBound: { [streamId]: offset3 },
        where: "'integration'",
        order: 'desc',
      })

      await getEventsInStreamAfterMs<Event>(REQUEST_OPTIONS_QUERY, QUERY_TIMEOUT, requestBody).then(
        (events) => {
          expect(events).toHaveLength(2)
          expect(events[0].offset).toBe(offset3)
          expect(events[1].offset).toBe(offset2)
        },
      )
    })
  })
})
