import {
  AxEventService,
  mkESFromTrial,
  Order,
  QueryRequest,
  QueryResponse,
} from '../../http-client'
import { run } from '../../util'
import { mySuite, publishRandom, testName, throwOnCb } from './utils.support.test'

const query = async (es: AxEventService, query: string): Promise<unknown[]> => {
  const result: unknown[] = []
  const offsets = await es.offsets()
  await es.query({ upperBound: offsets.present, query, order: Order.Asc }, (data) =>
    result.push(data.payload),
  )
  return result
}

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
        await es
          .query(req, throwOnCb('onData'))
          .then((x) => {
            throw new Error('invalid request succeeded: ' + x)
          })
          .catch((x) => {
            expect(x).toEqual({
              code: 'ERR_BAD_REQUEST',
              message: 'Invalid request. parsing StreamId at line 1 column 31',
            })
          })
      }))

    it('should return error if stream is not known', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const req: QueryRequest = {
          upperBound: {
            'aaaabbbbccccddddeeeeffffgggghhhhiiiijjjjkkk-0': 100,
          },
          query: "FROM 'integration' & 'test:1'",
          order: Order.Desc,
        }
        await es
          .query(req, throwOnCb('onData'))
          .then((x) => {
            throw new Error('invalid request succeeded: ' + x)
          })
          .catch((x) => {
            expect(x).toEqual({
              code: 'ERR_BAD_REQUEST',
              message:
                'Invalid request. Store error while reading: Upper bounds must be within the current offsets’ present.',
            })
          })
      }))

    it('should return events in ascending order and complete', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const pub1 = await publishRandom(es)
        const pub2 = await publishRandom(es)
        const request: QueryRequest = {
          lowerBound: { [pub2.stream]: pub1.offset - 1 },
          upperBound: { [pub2.stream]: pub2.offset },
          query: `FROM '${mySuite()}' & '${testName()}' & isLocal`,
          order: Order.Asc,
        }
        const data: QueryResponse[] = []
        await es.query(request, (x) => data.push(x))
        const pub1Idx = data.findIndex((x) => x.lamport === pub1.lamport)
        const pub2Idx = data.findIndex((x) => x.lamport === pub2.lamport)
        expect(data[pub1Idx]).toMatchObject(pub1)
        expect(data[pub2Idx]).toMatchObject(pub2)
        expect(pub1Idx).toBeLessThan(pub2Idx)
      }))

    it('should return events without bounds set in ascending order and complete', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const pub1 = await publishRandom(es)
        const pub2 = await publishRandom(es)
        const request: QueryRequest = {
          query: `FROM '${mySuite()}' & '${testName()}' & isLocal`,
          order: Order.Asc,
        }
        const data: QueryResponse[] = []
        await es.query(request, (x) => data.push(x))
        const pub1Idx = data.findIndex((x) => x.lamport === pub1.lamport)
        const pub2Idx = data.findIndex((x) => x.lamport === pub2.lamport)
        expect(data[pub1Idx]).toMatchObject(pub1)
        expect(data[pub2Idx]).toMatchObject(pub2)
        expect(pub1Idx).toBeLessThan(pub2Idx)
      }))

    it('should return events in descending order and complete', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const pub1 = await publishRandom(es)
        const pub2 = await publishRandom(es)
        const request: QueryRequest = {
          lowerBound: { [pub2.stream]: pub1.offset - 1 },
          upperBound: { [pub2.stream]: pub2.offset },
          query: `FROM '${mySuite()}' & '${testName()}' & isLocal`,
          order: Order.Desc,
        }
        const data: QueryResponse[] = []
        await es.query(request, (x) => data.push(x))
        const pub1Idx = data.findIndex((x) => x.lamport === pub1.lamport)
        const pub2Idx = data.findIndex((x) => x.lamport === pub2.lamport)
        expect(data[pub1Idx]).toMatchObject(pub1)
        expect(data[pub2Idx]).toMatchObject(pub2)
        expect(pub1Idx).toBeGreaterThan(pub2Idx)
      }))

    it('should return no events if upperBound is not in range', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const pub1 = await publishRandom(es)
        const request: QueryRequest = {
          lowerBound: { [pub1.stream]: Number.MAX_SAFE_INTEGER },
          upperBound: { [pub1.stream]: 0 },
          query: `FROM '${mySuite()}' & '${testName()}' & isLocal`,
          order: Order.Desc,
        }
        const data: QueryResponse[] = []
        await es.query(request, (x) => data.push(x))
        expect(data.length).toBe(0)
      }))

    it('should canonicalise tags correctly', () =>
      run(async (api) => {
        const es = await mkESFromTrial(api)
        await es.publish({
          data: [
            { tags: [mySuite(), 'query-canon', '\u{e9}'], payload: 1 },
            { tags: [mySuite(), 'query-canon', 'e\u{301}'], payload: 2 },
            { tags: [mySuite(), 'query-canon', 'ℌ'], payload: 3 },
            { tags: [mySuite(), 'query-canon', 'H'], payload: 4 },
            { tags: [mySuite(), 'query-canon', 'ﬁ'], payload: 5 },
            { tags: [mySuite(), 'query-canon', 'fi'], payload: 6 },
          ],
        })
        expect(
          await query(es, `FROM isLocal & "${mySuite()}" & "query-canon" & "\u{e9}"`),
        ).toEqual([1, 2])
        expect(
          await query(es, `FROM isLocal & "${mySuite()}" & "query-canon" & "e\u{301}"`),
        ).toEqual([1, 2])
        expect(await query(es, `FROM isLocal & "${mySuite()}" & "query-canon" & "ℌ"`)).toEqual([3])
        expect(await query(es, `FROM isLocal & "${mySuite()}" & "query-canon" & "H"`)).toEqual([4])
        expect(await query(es, `FROM isLocal & "${mySuite()}" & "query-canon" & "ﬁ"`)).toEqual([5])
        expect(await query(es, `FROM isLocal & "${mySuite()}" & "query-canon" & "fi"`)).toEqual([6])
      }))
  })
})
