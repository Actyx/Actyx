import { mkESFromTrial, Order, QueryRequest, QueryResponse } from '../../http-client'
import { run } from '../../util'
import { integrationTag, publishRandom, throwOnCb } from './utils.support.test'

describe('event service', () => {
  describe('query', () => {
    it('should return error if stream is not valid', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const req: QueryRequest = {
          upperBound: {
            'not-existing.0': 100,
          },
          query: "FROM 'integration' & 'test:1'",
          order: Order.Desc,
        }
        await es.query(req, throwOnCb('onData')).catch((x) => {
          expect(x).toEqual({
            code: 'ERR_BAD_REQUEST',
            message: 'Invalid request. parsing StreamId at line 1 column 31',
          })
        })
      }))

    /**
     * TODO: when query is in ASC lower and upper bounds are exclusive, when
     * in DESC it is lower exclusive and upper inclusive and the query never
     * completes
     */
    it('should return events in ascending order and complete', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const pub1 = await publishRandom(es)
        const pub2 = await publishRandom(es)
        const request: QueryRequest = {
          lowerBound: { [pub2.stream]: pub1.offset - 1 },
          upperBound: { [pub2.stream]: pub2.offset + 1 },
          query: `FROM '${integrationTag}'`,
          order: Order.Asc,
        }
        const data: QueryResponse[] = []
        await es.query(request, (x) => data.push(x))
        const pub1Idx = data.findIndex((x) => x.lamport === pub1.lamport)
        const pub2Idx = data.findIndex((x) => x.lamport === pub2.lamport)
        expect(data[pub1Idx]).toMatchObject(pub1)
        expect(data[pub2Idx]).toMatchObject(pub2)
        expect(pub1Idx < pub2Idx).toBe(true)
      }))

    it('should return events in descending order and complete', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const pub1 = await publishRandom(es)
        const pub2 = await publishRandom(es)
        const request: QueryRequest = {
          lowerBound: { [pub2.stream]: pub1.offset - 1 },
          upperBound: { [pub2.stream]: pub2.offset },
          query: `FROM '${integrationTag}'`,
          order: Order.Desc,
        }
        const data: QueryResponse[] = []
        //TODO: remove work around desc not completing
        await new Promise((resolve) => {
          es.query(request, (x) => data.push(x))
          setTimeout(resolve, 500)
        })
        const pub1Idx = data.findIndex((x) => x.lamport === pub1.lamport)
        const pub2Idx = data.findIndex((x) => x.lamport === pub2.lamport)
        expect(data[pub1Idx]).toMatchObject(pub1)
        expect(data[pub2Idx]).toMatchObject(pub2)
        expect(pub1Idx > pub2Idx).toBe(true)
      }))

    it('should return no events if upperBound is not in range', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const pub1 = await publishRandom(es)
        const request: QueryRequest = {
          lowerBound: { [pub1.stream]: Number.MAX_SAFE_INTEGER },
          upperBound: { [pub1.stream]: 0 },
          query: `FROM '${integrationTag}'`,
          order: Order.Desc,
        }
        const data: QueryResponse[] = []
        await new Promise((resolve) => {
          es.query(request, (x) => data.push(x))
          setTimeout(resolve, 500)
        })
        expect(data.length).toBe(0)
      }))
  })
})
