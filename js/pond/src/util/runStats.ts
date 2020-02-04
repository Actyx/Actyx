import { none, Option, some } from 'fp-ts/lib/Option'
import * as t from 'io-ts'
import { Observable } from 'rxjs'

// Copypasta to avoid weird circular dependency issues
const lookup = <V>(m: { [k: string]: V }, k: string): V | undefined => m[k]

// #region impl
type Duration = number
type Integer = number
// Copypasta to avoid weird circular dependency issues
type Timestamp = number
const Timestamp = {
  now: () => Date.now() * 1000,
}

const CounterMapEntry = t.type({
  count: t.number,
  last: t.number,
})
type CounterMapEntry = t.TypeOf<typeof CounterMapEntry>

const CounterMapInternal = t.record(t.string, CounterMapEntry)
type CounterMapInternal = t.TypeOf<typeof CounterMapInternal>

const CounterMapMut = t.record(t.string, t.tuple([t.number, t.number]))
type CounterMapMut = t.TypeOf<typeof CounterMapMut>

export const CounterMap = t.readonly(CounterMapMut)
export type CounterMap = t.TypeOf<typeof CounterMap>

const DurationStats = t.union([
  t.readonly(
    t.type({
      count: t.number,
      min: t.number,
      max: t.number,
      median: t.number,
      _90: t.number,
      _95: t.number,
      _99: t.number,
      pending: t.number,
      discarded: t.number,
    }),
  ),
  t.readonly(
    t.type({
      pending: t.number,
      discarded: t.number,
    }),
  ),
])
type DurationStats = t.TypeOf<typeof DurationStats>

const DurationMapMut = t.record(t.string, DurationStats)
type DurationMapMut = t.TypeOf<typeof DurationMapMut>

export const DurationMap = t.readonly(DurationMapMut)
export type DurationMap = t.TypeOf<typeof DurationMap>

type PendingOperationMap = { [key: number]: Integer }

/**
 * Internal representation for duration stats
 */
type DurationStats0 = {
  /**
   * List of durations in this interval
   */
  durations: Duration[]
  /**
   * Map of pending operations
   */
  pending: PendingOperationMap
  /**
   * Number of endPending that were discarded due to missing addPending
   */
  discarded: Integer
}

const addDurationStats = (stats: DurationStats0, duration: Duration): void => {
  const { durations } = stats
  // only sample the first 1000 in each time interval
  if (durations.length < 1000) {
    durations.push(duration)
  }
}

const addPendingOperation = (stats: DurationStats0, at: Timestamp): void => {
  const count = stats.pending[at] || 0
  stats.pending[at] = count + 1
}

const endPendingOperation = (stats: DurationStats0, from: Timestamp, to: Timestamp): void => {
  const count = stats.pending[from] || undefined
  if (count === undefined || count <= 0) {
    stats.discarded += 1
    return
  }
  const newCount = count - 1
  if (newCount === 0) {
    delete stats.pending[from]
  } else {
    stats.pending[from] = newCount
  }
  addDurationStats(stats, to - from)
}

const createEntry = (duration: Duration): DurationStats0 => ({
  durations: [duration],
  pending: {},
  discarded: 0,
})

const createEntryAt = (at: Timestamp): DurationStats0 => ({
  durations: [],
  pending: { [at]: 1 },
  discarded: 0,
})

const createDiscardedEntry = (): DurationStats0 => ({
  durations: [],
  pending: {},
  discarded: 1,
})

const getDurationStats = (stats: DurationStats0): Option<DurationStats> => {
  const count = stats.durations.length
  const pending = Object.values(stats.pending).reduce((acc, v) => acc + v, 0)
  const discarded = stats.discarded
  if (count === 0) {
    if (pending !== 0 || discarded !== 0) return some({ pending, discarded })
    else return none
  }
  const d = stats.durations.sort((a, b) => a - b)
  const min = d[0]
  const median = d[Math.floor(count / 2)]
  const _90 = d[Math.floor(count * 0.9)]
  const _95 = d[Math.floor(count * 0.95)]
  const _99 = d[Math.floor(count * 0.99)]
  const max = d[count - 1]

  // clear the stats, this is reported per interval
  stats.durations = []
  stats.discarded = 0

  return some({
    count,
    pending,
    discarded,
    min,
    median,
    _90,
    _95,
    _99,
    max,
  })
}

type DurationMapBuilder = {
  [key: string]: DurationStats0
}

const mkCounters = (): Counters => {
  const counters: CounterMapInternal = {}
  const add = (name: string, count?: Integer) => {
    const entry: CounterMapEntry | undefined = lookup(counters, name)
    if (entry === undefined) {
      counters[name] = { count: count || 1, last: 0 }
    } else {
      entry.count += count || 1
    }
  }
  const current = () => {
    const result: CounterMapMut = {}
    Object.entries(counters).forEach(([name, entry]) => {
      result[name] = [entry.count, entry.count - entry.last]
      entry.last = entry.count
    })
    return result
  }
  return {
    add,
    current,
  }
}

const createDurationStats = (): Durations => {
  const durations: DurationMapBuilder = {}
  const add = (name: string, duration: Duration): void => {
    const entry: DurationStats0 | undefined = lookup(durations, name)
    if (entry === undefined) {
      durations[name] = createEntry(duration)
    } else {
      addDurationStats(entry, duration)
    }
  }
  const start = (name: string, at: Timestamp): void => {
    const entry: DurationStats0 | undefined = lookup(durations, name)
    if (entry === undefined) {
      durations[name] = createEntryAt(at)
    } else {
      addPendingOperation(entry, at)
    }
  }
  const end = (name: string, from: Timestamp, to: Timestamp): void => {
    const entry: DurationStats0 | undefined = lookup(durations, name)
    if (entry === undefined) {
      durations[name] = createDiscardedEntry()
      return
    } else {
      endPendingOperation(entry, from, to)
    }
  }
  const getAndClear = (): DurationMap => {
    const result: DurationMapMut = {}
    Object.entries(durations).forEach(([key, value]) => {
      getDurationStats(value).map(stats => (result[key] = stats))
      if (Object.keys(value.pending).length === 0) {
        // If nothing is pending then this object is now useless, remove it.
        // This metric may be repopulated in the next interval.
        delete durations[key]
      }
    })
    return result
  }
  return {
    add,
    getAndClear,
    start,
    end,
  }
}

const profileSync = <T>(durations: Durations) => (name: string) => (block: () => T): T => {
  const t0 = Timestamp.now()
  let result: T
  try {
    result = block()
    durations.add(name, Timestamp.now() - t0)
    return result
  } catch (e) {
    durations.add(name, Timestamp.now() - t0)
    throw e
  }
}

const profileObservable = <T>(durations: Durations, counters: Counters) => (
  name: string,
  n?: number,
) => (inner: Observable<T>): Observable<T> => {
  const maxCount = n || 1
  return new Observable<T>(subscriber => {
    let t0 = Timestamp.now()
    let count = 0
    durations.start(name, t0)
    return inner.subscribe({
      next: value => {
        count++
        if (count <= maxCount) {
          const t1 = Timestamp.now()
          durations.end(name, t0, t1)
          t0 = t1
        }
        if (count < maxCount) {
          durations.start(name, t0)
        }
        subscriber.next(value)
      },
      error: reason => {
        counters.add(`errprof-${name}`)
        count++
        if (count <= maxCount) {
          durations.end(name, t0, Timestamp.now())
        }
        subscriber.error(reason)
      },
      complete: () => {
        count++
        if (count <= maxCount) {
          durations.end(name, t0, Timestamp.now())
        }
        subscriber.complete()
      },
    })
  })
}

const GaugeMapMut = t.record(
  t.string,
  t.type({
    last: t.number,
    max: t.number,
  }),
)
type GaugeMapMut = t.TypeOf<typeof GaugeMapMut>
export const GaugeMap = t.readonly(GaugeMapMut)
export type GaugeMap = t.TypeOf<typeof GaugeMap>

const mkGauges = (): Gauges => {
  const gauges: GaugeMapMut = {}
  const set = (name: string, value: Integer) => {
    const old = gauges[name] || { last: value, max: value }
    const next = { last: value, max: Math.max(old.max, value) }
    gauges[name] = next
  }
  const current = () => ({ ...gauges })
  return {
    set,
    current,
  }
}

const createRunStats = (): RunStats => {
  const durations = createDurationStats()
  const counters = mkCounters()
  const profile: ProfileMethods = {
    profileSync: profileSync(durations),
    profileObservable: profileObservable(durations, counters),
  }
  const gauges = mkGauges()
  return {
    counters,
    durations,
    profile,
    gauges,
  }
}
// #endregion

/**
 * Methods to work with simple and rather cheap invocation counters
 */
export type Counters = Readonly<{
  /**
   * Increment a named counter value. Default increment is one.
   */
  add: (name: string, count?: Integer) => void
  /**
   * Get all current counter values. Returns an immutable copy.
   */
  current: () => CounterMap
}>

/**
 * Methods to work with duration statistics
 */
export type Durations = Readonly<{
  /**
   * Start a long-running operation for a metric name. The operation will be
   * ongoing until `end()` is called!
   */
  start: (name: string, at: Timestamp) => void
  /**
   * End a long-running operation for a metric name. This call only makes sense
   * in conjunction with a preceding start.
   *
   * The from parameter must be the timestamp at which the operation was started,
   * to allow the operation to be properly ended.
   *
   * See profileSync and profileObservable for a more convenient way to use these methods
   */
  end: (name: string, from: Timestamp, to: Timestamp) => void
  /**
   * Add a duration to a named duration statistic.
   * The duration should be positive, but this is not enforced
   */
  add: (name: string, duration: Duration) => void
  /**
   * Get all current duration statistics. Returns an immutable copy.
   */
  getAndClear: () => DurationMap
}>

/**
 * Convenience methods for profiling synchronous and asynchronous operations
 */
export type ProfileMethods = Readonly<{
  /**
   * Profile a synchronous block of code. Will add the duration it took to execute
   * the block to the statistics, regardless of whether the block terminates normally
   * or with an exception.
   */
  profileSync: <T>(name: string) => (block: () => T) => T
  /**
   * Profile an observable. Will measure the time between the materialisation of the
   * observable and the first element, and then the time between elements, up to a
   * configurable number of elements.
   *
   * The default is to track until the first element arrives or the observable completes
   * with a completion or an error. To track indefinitely, pass Number.MAX_SAFE_INTEGER.
   *
   * This is meant to be used with the rxjs pipe operator.
   */
  profileObservable: <T>(name: string, n?: Integer) => (inner: Observable<T>) => Observable<T>
}>

/**
 * Methods to work with simple gauges
 */
export type Gauges = Readonly<{
  /**
   * Set a gauge value.
   */
  set: (name: string, value: Integer) => void

  /**
   * Get all current gauge values. Returns an immutable copy.
   */
  current: () => GaugeMap
}>

export interface RunStats {
  readonly counters: Counters
  readonly durations: Durations
  readonly profile: ProfileMethods
  readonly gauges: Gauges
}

export const RunStats = {
  create: createRunStats,
}

/**
 * Global statistics singleton.
 *
 * In the same file to avoid circular dependency issues.
 */
export const runStats = RunStats.create()
