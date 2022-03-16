/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2020 Actyx AG
 */
import * as moment from 'moment'
import { Timestamp } from '../'
import { TaggedIndex } from '../types'
import { isEven, SnapshotScheduler } from './snapshotScheduler'

type Event = Readonly<{ timestamp: Timestamp; tag?: string }>

/**
 * number of trailing binary zeroes for an integer
 */
const numberOfTrailingZeros = (x: number): number => {
  if (x === 0) {
    // pretend we got proper integers
    return 64
  }

  return Math.log2(x & -x)
}

describe('numberOfTrailingZeros', () => {
  expect(numberOfTrailingZeros(0)).toEqual(64)
  expect(numberOfTrailingZeros(1)).toEqual(0)
  expect(numberOfTrailingZeros(1024)).toEqual(10)
})

describe('grid', () => {
  const parseDate = (text: string): Timestamp => Timestamp.of(moment(text).unix() * 1000000)
  const events: Event[] = []
  const timestamp = parseDate('2018-09-01T00:00:01Z')
  // the past
  const n = 20000
  for (let i = 0; i < n; i++) {
    events.push({ timestamp })
  }
  // the present
  events.push({ timestamp: parseDate('2019-09-01T00:00:01Z') })
  events.push({ timestamp: parseDate('2019-09-01T00:00:02Z') })
  events.push({ timestamp: parseDate('2019-09-01T00:00:03Z') })

  const getExpected = (initialCycle: number, limit: number) => {
    const expected: Record<string, TaggedIndex> = {}

    for (let i = limit; i < n; i++) {
      const overallIndex = i + initialCycle

      const level = numberOfTrailingZeros(overallIndex)
      if (level >= 10 && isEven(level)) {
        expected[`${level}`] = {
          tag: `${level}`,
          i,
          persistAsLocalSnapshot: true,
        }
      }
    }

    return Object.values(expected).sort(TaggedIndex.ord.compare)
  }

  it('should properly tag events for snapshotting', () => {
    const expected = getExpected(1, 0)

    const scheduler = SnapshotScheduler.create(10)
    const tags = scheduler.getSnapshotLevels(1, events, 0)
    expect(tags.length).toEqual(3)
    expect(tags).toEqual(expected)
  })

  it('should respect snapshot limits', () => {
    const limit = 16150 // should give levels 1024 and 4048
    const expected = getExpected(1, limit)

    const scheduler = SnapshotScheduler.create(10)
    const tags = scheduler.getSnapshotLevels(1, events, limit)
    expect(tags.length).toEqual(2)
    expect(tags).toEqual(expected)
  })
})
