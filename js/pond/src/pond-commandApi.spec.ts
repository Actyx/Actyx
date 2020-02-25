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
    return resP
      .then(() => {
        return expect(statesP).resolves.toEqual({ hashes: {} })
      })
      .then(pond.dispose)
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
    await expect(resP.then(() => navP)).resolves.toEqual('BOO')
    await pond.dispose()
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
      const pond = await Pond.test()
      const probeP = pond
        .observe(commandProbe, FishName.of('probe'))
        .take(7)
        .toArray()
        .toPromise()

      const feed = (id: number) =>
        pond.feed(asyncTestFish, fooFishName)({ type: 'slow', id, target })

      await Observable.range(0, 3)
        .mergeMap(i => feed(i))
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

      return pond.dispose()
    },
    15000,
  )
})
