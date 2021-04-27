/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Offset, OffsetMap } from '@actyx/sdk'
import { Observable } from 'rxjs'
import { swarmState } from './swarmState'

describe('swarmState', () => {
  it('should aggregate info from ourselves', async () => {
    const ownMap: OffsetMap = {
      myself: Offset.of(10),
      a: Offset.of(10),
      c: Offset.of(100), // only we know this guy
    }
    const swarmMap: OffsetMap = {
      myself: Offset.of(5), // swarm knows us, but with a lower PSN
      a: Offset.of(20), // we have this, but with a lower PSN
      b: Offset.of(30), // we don't have this at all
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
        myself: Offset.of(10),
      },
      {
        myself: Offset.of(5),
      },
    ]
    const swarmMaps: ReadonlyArray<OffsetMap> = [
      {
        a: Offset.of(30),
      },
      {
        a: Offset.of(20),
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
