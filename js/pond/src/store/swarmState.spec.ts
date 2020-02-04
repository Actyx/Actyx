import { Observable } from 'rxjs'
import { OffsetMap } from '../eventstore'
import { Psn } from '../types'
import { swarmState } from './swarmState'

describe('swarmState', () => {
  it('should aggregate info from ourselves', async () => {
    const ownMap: OffsetMap = {
      myself: Psn.of(10),
      a: Psn.of(10),
      c: Psn.of(100), // only we know this guy
    }
    const swarmMap: OffsetMap = {
      myself: Psn.of(5), // swarm knows us, but with a lower PSN
      a: Psn.of(20), // we have this, but with a lower PSN
      b: Psn.of(30), // we don't have this at all
    }
    const own = Observable.of(ownMap)
    const swarm = Observable.of(swarmMap)
    const result = await swarmState(own, swarm)
      .take(2)
      .toPromise()
    expect(result).toMatchInlineSnapshot(
      `
Object {
  "nodes": Immutable.Map {
    "myself": Object {
      "own": 10,
      "swarm": 5,
    },
    "a": Object {
      "own": 10,
      "swarm": 20,
    },
    "c": Object {
      "own": 100,
    },
    "b": Object {
      "swarm": 30,
    },
  },
}
`,
    )
  })
  it('should only increase psns', async () => {
    const ownMaps: ReadonlyArray<OffsetMap> = [
      {
        myself: Psn.of(10),
      },
      {
        myself: Psn.of(5),
      },
    ]
    const swarmMaps: ReadonlyArray<OffsetMap> = [
      {
        a: Psn.of(30),
      },
      {
        a: Psn.of(20),
      },
    ]
    const own = Observable.from(ownMaps)
    const swarm = Observable.from(swarmMaps)
    const result = await swarmState(own, swarm)
      .take(4)
      .toPromise()
    expect(result).toMatchInlineSnapshot(
      `
Object {
  "nodes": Immutable.Map {
    "myself": Object {
      "own": 10,
    },
    "a": Object {
      "swarm": 30,
    },
  },
}
`,
    )
  })
})
