/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { TaggedIndex, Timestamp } from '../types'
import { RWPartialRecord, valuesOf } from '../util'

export type SnapshotLevels = ReadonlyArray<TaggedIndex>

export const isEven = (x: number): boolean => x % 2 === 0

/**
 * Mark indices at which a snapshot should be taken with a tag indicating the level of the snapshot
 *
 * @param cycle cycle of the first event. Cycles start with 1!
 * @param events events to tag with snapshot markers
 * @param from which point on to check the array for snapshots
 *
 * @returns a sparse array of the same size as events, with the tags for the corresponding events.
 */
export type GetSnapshotLevels = <T extends HasTimestamp>(
  cycle: number,
  events: ReadonlyArray<T>,
  from: number,
) => SnapshotLevels

/**
 * Qualify previously scheduled (via GetSnapshotLevels) snapshots for application/storage,
 * i.e. decide whether they are old enough to be applied to the fishEventStore without too much
 * danger of shattering soon.
 *
 * @param scheduledSnapshot snapshot that was scheduled previously
 * @param latestState the latest known event, to calculate how old the snapshot is
 *
 * @returns whether to apply the `scheduledSnapshot` to the FES
 */
export type IsEligibleForStorage = <T extends HasTimestamp>(
  scheduledSnapshot: T,
  latestState: T,
) => boolean

export type SnapshotScheduler = {
  minEventsForSnapshot: number
  getSnapshotLevels: GetSnapshotLevels
  isEligibleForStorage: IsEligibleForStorage
}

/**
 * Minimum age of a state before it will be considered for a snapshot.
 *
 * Everything younger than this will be considered "the present", where
 * time travel can happen at any time and storing snapshots would be wasteful.
 */
export const minSnapshotAge = 60 * 60 * 1000 * 1000

/**
 * The part of an event that we care about for the purpose of this method
 *
 * Not taking whole events makes it easier to write tests
 */
export type HasTimestamp = Readonly<{
  timestamp: Timestamp
}>

const multiplesBetween = (low: number, high: number) => (n: number) => {
  let m = Math.floor(high / n)

  const res = []
  while (m * n >= low) {
    res.push(m * n)
    m -= 1
  }

  return res
}

const getSnapshotLevels = (minLevel: number) => (
  initialCycle: number,
  events: ReadonlyArray<HasTimestamp>,
  from: number,
): ReadonlyArray<TaggedIndex> => {
  const result: RWPartialRecord<number, TaggedIndex> = {}

  const maxCycle = initialCycle + events.length - 1
  const getIndices = multiplesBetween(initialCycle + from, maxCycle)

  const maxExpectedLevel = Math.floor(Math.log2(maxCycle))

  let level = isEven(maxExpectedLevel) ? maxExpectedLevel : maxExpectedLevel - 1
  for (; level >= minLevel; level -= 2) {
    // E.g. level=10 means all multiples of 1024 qualify
    const levelDivisor = Math.pow(2, level)

    // We try to find the highest applicable index not already taken by a higher level snapshot.
    for (const index of getIndices(levelDivisor)) {
      const i = index - initialCycle
      // Don’t snapshot the same index twice.
      if (result[i] === undefined) {
        result[i] = { tag: `${level}`, i, persistAsLocalSnapshot: true }
        break
      }
    }
  }

  const v: TaggedIndex[] = valuesOf(result)
  return v.sort(TaggedIndex.ord.compare)
}

export const isEligibleForStorage: IsEligibleForStorage = (snap, latest) => {
  const tMax = latest.timestamp

  const maxCrystal = tMax - minSnapshotAge
  return maxCrystal >= snap.timestamp
}

const mkSnapshotScheduler = (minLevel: number): SnapshotScheduler => ({
  minEventsForSnapshot: Math.pow(2, minLevel + 2), // Don't snapshot too early. (This is used for streaming hydration.)
  getSnapshotLevels: getSnapshotLevels(minLevel),
  isEligibleForStorage,
})

export const SnapshotScheduler = {
  create: mkSnapshotScheduler,
}
