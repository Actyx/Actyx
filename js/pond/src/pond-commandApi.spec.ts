import * as fetchMock from 'fetch-mock'
import { Observable } from 'rxjs'
import { Target } from '.'
import { asyncTestFish } from './asyncCommandTestFish.support.test'
import { Pond } from './pond'
import { commandProbe } from './testkit/commandProbe'
import { FishName } from './types'

const fooFishName = FishName.of('foo')

afterEach(() => fetchMock.restore())

describe('Pond', () => {
  it('should gracefully handle broken command handlers', async () => {
    const pond = await Pond.mock()
    const statesP = pond
      .observe(asyncTestFish, fooFishName)
      .take(1)
      .toPromise()
    const resP = pond
      .feed(asyncTestFish, fooFishName)({
        type: 'broken',
      })
      .toPromise()
    return resP.then(() => {
      return expect(statesP).resolves.toEqual({ hashes: {} })
    })
  })

  // FIXME: ipfs is not a thing at all
  it.skip('should allow async fishes', async () => {
    // // uncomment this and comment out the mocking to do a real request
    // // (requires locally running ipfs node)
    // // const x: any = global
    // // x.fetch = require('node-fetch')
    // fetchMock.get('http://localhost:8080/ipfs/QmP1q9NDoYaV5p7EfbELK6FS9DYx629jFD14KWF65RQSmQ', {
    //   a: 1,
    // })
    // const pond = await Pond.mock()
    // const name = FishName.of('foo')
    // const statesP = pond
    //   .observe(asyncTestFish, name)
    //   .take(2)
    //   .toArray()
    //   .toPromise()
    // return resP.then(() => {
    //   return expect(statesP).resolves.toEqual([
    //     { hashes: {} },
    //     {
    //       hashes: {
    //         'http://localhost:8080/ipfs/QmP1q9NDoYaV5p7EfbELK6FS9DYx629jFD14KWF65RQSmQ':
    //           '015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862',
    //       },
    //     },
    //   ])
    // })
  })

  it('should allow async fishes to send commands', async () => {
    const pond = await Pond.mock()
    const resP = pond
      .feed(asyncTestFish, fooFishName)({
        type: 'send',
        target: Target.of(commandProbe, FishName.of('probe')),
        cmd: 'BOO',
      })
      .toPromise()
    const navP = pond
      .observe(commandProbe, FishName.of('probe'))
      .take(2)
      .toPromise()
    return expect(resP.then(() => navP)).resolves.toEqual('BOO')
  })

  // FIXME: ipfs is not a thing anymore
  it.skip('should allow testing async fishes using snapshot tests', () => {
    // const httpGet = {
    //   'http://localhost:8080/ipfs/QmP1q9NDoYaV5p7EfbELK6FS9DYx629jFD14KWF65RQSmQ': {
    //     a: 1,
    //   },
    // }
    // const executor = TestCommandExecution({ httpGet, httpPost: {} })
    // const { onCommand, initialState } = asyncTestFish
    // const state0 = initialState(FishName.of('test'), SourceId.of('X')).state
    // expect(
    //   executor(
    //     onCommand(state0, {
    //       type: 'import',
    //       url: 'http://localhost:8080/ipfs/QmP1q9NDoYaV5p7EfbELK6FS9DYx629jFD14KWF65RQSmQ',
    //     }),
    //   ),
    // ).toMatchSnapshot()
  })

  it(
    'should execute async commands strictly in sequence',
    async () => {
      // simulate a slow web site

      fetchMock.get('http://slow.com', () =>
        Observable.timer(200)
          .mapTo(200)
          .toPromise(),
      )
      const target = Target.of(commandProbe, FishName.of('probe'))
      const pond = await Pond.mock()
      const probeP = pond
        .observe(commandProbe, FishName.of('probe'))
        .take(7)
        .toArray()
        .toPromise()
      const feed = (id: number) =>
        pond
          .feed(asyncTestFish, fooFishName)({ type: 'slow', id, target })
          .toPromise()

      await Observable.range(0, 3)
        .concatMap(i => feed(i))
        .toPromise()
      const result = await probeP
      expect(result).toEqual([
        null,
        'starting 0',
        'ending 0',
        'starting 1',
        'ending 1',
        'starting 2',
        'ending 2',
      ])
    },
    15000,
  )
})
