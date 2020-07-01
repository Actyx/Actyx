/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Either, right } from 'fp-ts/lib/Either'
import { contramap, Ord, ordNumber, ordString } from 'fp-ts/lib/Ord'
import { Ordering } from 'fp-ts/lib/Ordering'
import * as t from 'io-ts'
import { Observable } from 'rxjs'
import { CommandApi } from './commandApi'
import { Event, OffsetMap } from './eventstore/types'
import { EnvelopeFromStore } from './store/util'
import { Subscription } from './subscription'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const isString = (x: any): x is string => typeof x === 'string'
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const isNumber = (x: any): x is number => typeof x === 'number'
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const isBoolean = (x: any): x is boolean => typeof x === 'boolean'

export type Semantics = string

const internalSemantics = (s: string): Semantics => `internal-${s}` as Semantics
export const Semantics = {
  of(name: string): Semantics {
    if (name.startsWith('jelly-')) {
      throw new Error('Name must not start with jelly-')
    }
    if (name.startsWith('internal-')) {
      throw new Error('Name must not start with internal-')
    }
    return name as Semantics
  },
  jelly: (s: string): Semantics => `jelly-${s}` as Semantics,
  isJelly: (s: Semantics): boolean => s.startsWith('jelly-'),
  internal: internalSemantics,
  isInternal: (s: Semantics): boolean => s.startsWith('internal-'),
  none: internalSemantics('nofish'),
  FromString: new t.Type<Semantics, string>(
    'SemanticsFromString',
    (x): x is Semantics => isString(x),
    (x, c) => t.string.validate(x, c).map(s => s as Semantics),
    x => x,
  ),
}

export type FishName = string
export const FishName = {
  of: (s: string): FishName => s as FishName,
  none: 'internal-nofish' as FishName,
  FromString: new t.Type<FishName, string>(
    'FishNameFromString',
    (x): x is FishName => isString(x),
    (x, c) => t.string.validate(x, c).map(s => s as FishName),
    x => x,
  ),
}

export type Tags = ReadonlyArray<string>
type TagsOnWire = ReadonlyArray<string> | undefined
export const Tags = new t.Type<Tags, TagsOnWire>(
  'TagsSetFromArray',
  (x): x is Tags => x instanceof Array && x.every(isString),
  // Rust side for now expresses empty tag arrays as omitting the field
  (x, c) => (x === undefined ? right([]) : t.readonlyArray(t.string).validate(x, c)),
  // Sending empty arrays is fine, though
  x => x,
)

export type SourceId = string
const mkSourceId = (text: string): SourceId => text as SourceId
export const randomBase58: (digits: number) => string = (digits: number) => {
  const base58 = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'.split('')

  let result = ''
  let char

  while (result.length < digits) {
    char = base58[(Math.random() * 57) >> 0]
    result += char
  }
  return result
}
export const SourceId = {
  /**
   * Creates a SourceId from a string
   */
  of: mkSourceId,
  /**
   * Creates a random SourceId with the given number of digits
   */
  random: (digits?: number) => mkSourceId(randomBase58(digits || 11)),
  FromString: new t.Type<SourceId, string>(
    'SourceIdFromString',
    (x): x is SourceId => isString(x),
    (x, c) => t.string.validate(x, c).map(s => s as SourceId),
    x => x,
  ),
}

export type Lamport = number
const mkLamport = (value: number): Lamport => value as Lamport
export const Lamport = {
  of: mkLamport,
  zero: mkLamport(0),
  FromNumber: new t.Type<Lamport, number>(
    'LamportFromNumber',
    (x): x is Lamport => isNumber(x),
    (x, c) => t.number.validate(x, c).map(s => mkLamport(s)),
    x => x,
  ),
}

export type Psn = number
const mkPsn = (psn: number): Psn => psn as Psn
export const Psn = {
  of: mkPsn,
  zero: mkPsn(0),
  /**
   * A value that is below any valid Psn
   */
  min: mkPsn(-1),
  /**
   * A value that is above any valid Psn
   */
  max: mkPsn(Number.MAX_SAFE_INTEGER),
  FromNumber: new t.Type<Psn, number>(
    'PsnFromNumber',
    (x): x is Psn => isNumber(x),
    (x, c) => t.number.validate(x, c).map(s => s as Psn),
    x => x,
  ),
}

export type Timestamp = number
const mkTimestamp = (time: number): Timestamp => time as Timestamp
const formatTimestamp = (timestamp: Timestamp): string => new Date(timestamp / 1000).toISOString()
const secondsPerDay = 24 * 60 * 60
export const Timestamp = {
  of: mkTimestamp,
  zero: mkTimestamp(0),
  maxSafe: mkTimestamp(Number.MAX_SAFE_INTEGER),
  now: (now?: number) => mkTimestamp((now || Date.now()) * 1e3),
  format: formatTimestamp,
  toSeconds: (value: Timestamp) => value / 1e6,
  toMilliseconds: (value: Timestamp): Milliseconds => Milliseconds.of(value / 1e3),
  toDate: (value: Timestamp): Date => new Date(value / 1e3),
  fromDate: (date: Date): Timestamp => mkTimestamp(date.valueOf() * 1e3),
  fromDays: (value: number) => Timestamp.fromSeconds(secondsPerDay * value),
  fromSeconds: (value: number) => mkTimestamp(value * 1e6),
  fromMilliseconds: (value: number) => mkTimestamp(value * 1e3),
  min: (...values: Timestamp[]) => mkTimestamp(Math.min(...values)),
  max: (values: Timestamp[]) => mkTimestamp(Math.max(...values)),
  FromNumber: new t.Type<Timestamp, number>(
    'TimestampFromNumber',
    (x): x is Timestamp => isNumber(x),
    (x, c) => t.number.validate(x, c).map(s => s as Timestamp),
    x => x,
  ),
}

export type Milliseconds = number
const mkMilliseconds = (time: number): Milliseconds => time as Milliseconds
export const Milliseconds = {
  of: mkMilliseconds,
  fromDate: (date: Date): Milliseconds => mkMilliseconds(date.valueOf()),
  zero: mkMilliseconds(0),
  now: (now?: number): Milliseconds => mkMilliseconds(now || Date.now()),
  toSeconds: (value: Milliseconds): number => value / 1e3,
  toTimestamp: (value: Milliseconds): Timestamp => Timestamp.of(value * 1e3),
  fromSeconds: (value: number) => mkMilliseconds(value * 1e3),
  fromMinutes: (value: number) => mkMilliseconds(value * 1e3 * 60),
  // Converts millis or micros to millis
  // Note: This is a stopgap until we fixed once and for all this mess.
  fromAny: (value: number): Milliseconds => {
    const digits = Math.floor(Math.abs(value)).toString().length
    return Milliseconds.of(digits <= 13 ? value : value / 1e3)
  },
  FromNumber: new t.Type<Milliseconds, number>(
    'MilisecondsFromString',
    (x): x is Milliseconds => isNumber(x),
    (x, c) => t.number.validate(x, c).map(mkMilliseconds),
    x => x,
  ),
}

/**
 * The source of an event stream: a single localized fish instance
 * characterised by its semantic name, instance name, pond sourceId.
 */
export type Source = Readonly<{
  semantics: Semantics
  name: FishName
  sourceId: SourceId
}>

/**
 * Turn a Source into its flat string representation.
 *
 * Note that this will not properly escape slashes in the parameters,
 * so it is only useful for debugging!
 */
function streamName(source: Source): string {
  return `${source.semantics}/${source.name}/${source.sourceId}`
}

/**
 * The target of a SendCommand effect, characterised by its fish type
 * and instance name; sending to non-local pond is not supported, hence
 * no sourceId.
 */
export type Target<C> = Readonly<{
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  semantics: FishType<C, any, any>
  name: FishName
}>

export type SendCommand<C> = Readonly<{
  target: Target<C>
  command: C
}>

export type HttpResponseSuccess = {
  status: 'success'
  message?: string
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  data: any
}

export type HttpResponseError = {
  status: 'error' | 'networkError'
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  error: any
}

export type HttpResponse = HttpResponseSuccess | HttpResponseError

export type Emit<E> = Readonly<{
  tags: string[]
  payload: E
}>
export type TaggedEvents<E> = ReadonlyArray<Emit<E>>

export type SyncCommandResult<E> = ReadonlyArray<E>

export type AsyncCommandResult<E> = CommandApi<ReadonlyArray<E>>

export type CommandResult<E> = SyncCommandResult<E> | AsyncCommandResult<E> // | TaggedEvents<E>

export const CommandResult = {
  fold: <E, R>(result: CommandResult<E>) => (handlers: {
    sync: (value: SyncCommandResult<E>) => R
    async: (value: AsyncCommandResult<E>) => R
    none: () => R
  }) => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const ar: any = result
    if (Array.isArray(ar)) {
      return handlers.sync(ar) // just an array of events
    } else if (ar !== undefined && ar.chain !== undefined) {
      return handlers.async(ar) // probably a CommandApi
    } else {
      return handlers.none()
    }
  },
}

/**
 * Metadata wrapper for an event. This contains all information known to
 * the event store about this event and is passed into OnEvent so that
 * the receiving fish can distinguish between events from different sources
 * it has subscribed to.
 */
export type Envelope<E> = {
  readonly source: Source
  readonly lamport: Lamport
  readonly timestamp: Timestamp // Number of microseconds since the unix epoch. Date.now() * 1000
  readonly payload: E
}

const zeroKey: EventKey = {
  lamport: Lamport.zero,
  // Cannot use empty source id, store rejects.
  sourceId: SourceId.of('!'),
  psn: Psn.of(0),
}

const keysEqual = (a: EventKey, b: EventKey): boolean =>
  a.lamport === b.lamport && a.sourceId === b.sourceId

const keysCompare = (a: EventKey, b: EventKey): Ordering => {
  const lamportOrder = ordNumber.compare(a.lamport, b.lamport)
  if (lamportOrder !== 0) {
    return lamportOrder
  }
  return ordString.compare(a.sourceId, b.sourceId)
}

/**
 * Order for event keys
 *
 * Order is [timestamp, sourceId, psn]. Envent keys are considered equal when `timestamp`,
 * `sourceId` and `psn` are equal.
 */
const ordEventKey: Ord<EventKey> = {
  equals: keysEqual,
  compare: keysCompare,
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const keyFromEnvelope = (env: EnvelopeFromStore<any>): EventKey => ({
  lamport: env.lamport,
  sourceId: env.source.sourceId,
  psn: env.psn,
})
const formatEventKey = (key: EventKey): string => `${key.lamport}/${key.sourceId}`

export const EventKey = {
  zero: zeroKey,
  ord: ordEventKey,
  fromEnvelope: keyFromEnvelope,
  format: formatEventKey,
}
export const EventKeyIO = t.readonly(
  t.type({
    lamport: Lamport.FromNumber,
    psn: Psn.FromNumber,
    sourceId: SourceId.FromString,
  }),
)

export type EventKey = t.TypeOf<typeof EventKeyIO>

export type SendCommandEffect<C> = {
  type: 'sendCommand'
  command: SendCommand<C>
}

export type SendSelfCommand<C> = { readonly type: 'sendSelfCommand'; readonly command: C }
export function mkSendSelf<C>(command: C): SendSelfCommand<C> {
  return { type: 'sendSelfCommand', command }
}

export type PublishState<P> = { readonly type: 'publish'; readonly state: P }
export function mkPublish<P>(p: P): PublishState<P> {
  return { type: 'publish', state: p }
}

export type InitialState<S> = (
  fishName: string,
  sourceId: SourceId,
) => Readonly<{
  state: S
  subscriptions?: ReadonlyArray<Subscription>
}>

// Combine the existing ("old") state and next event into a new state.
// The returned value may be something completely new, or a mutated version of the input state.
export type OnEvent<S, E> = (state: S, event: Envelope<E>) => S

export type OnCommand<S, C, E> = (
  state: S,
  command: C,
) => SyncCommandResult<E> | AsyncCommandResult<E>

export const publishState = <S, C, P>(f: (state: S) => P): StateSubscription<S, C, P> => ({
  name: 'publishState',
  create: pond => pond.observeSelf().map(s => mkPublish(f(s))),
})

export type OnStateChange<S, C, P> = (pond: PondObservables<S>) => Observable<StateEffect<C, P>>
export type OnStateChangeCompanion = {
  publishState: <S, P>(f: (state: S) => P) => OnStateChange<S, never, P>
  publishPrivateState: <S>() => OnStateChange<S, never, S>
  noPublish: <S>() => OnStateChange<S, never, never>
}
export const OnStateChange: OnStateChangeCompanion = {
  publishState: <S, P>(f: (state: S) => P) => (pond: PondObservables<S>) =>
    pond.observeSelf().map(s => mkPublish(f(s))),
  publishPrivateState: <S>() => (pond: PondObservables<S>) => pond.observeSelf().map(mkPublish),
  noPublish: () => () => Observable.never(),
}

export type SemanticSnapshot<E> = (
  name: FishName,
  sourceId: SourceId,
) => (ev: Envelope<E>) => boolean

export type SnapshotFormat<S, Serialized> = {
  /**
   * This number must be increased whenever:
   *
   * - code changes are made that affect the computed private state
   * - private state type definition is changed
   * - subscription set is changed
   *
   * The version number may remain the same in those rare cases where the new
   * code will seamlessly work with the old snapshots, or if the deserialize
   * function recognizes old snapshot format and converts them to the new one.
   */
  version: number
  /**
   * This function is used to transform the private state into an object that
   * can be serialized using `JSON.stringify()`. In many cases this can be the
   * identity function. Please note that while e.g. immutable Map serializes
   * itself into json automatically, you should still explicitly call `.toJS()`
   * in case the serialization is something else than JSON.stringify(), like
   * e.g. CBOR encoding or storing in indexeddb.
   *
   * In case of function objects within the private state this needs to ensure
   * that the functions can be properly recreated by persisting the values
   * that are captured by the closures.
   */
  serialize: (state: S) => Serialized
  /**
   * A snapshot comes back from the store as the JS object that `serialize`
   * produced, and this function needs to restore it into a proper private
   * state. Please note that while e.g. immutable Map serializes to a proper
   * object by itself, deserialization does NOT yield an immutable Map but
   * just a plain object, so `deserialize` needs to use `Map(obj)`
   * constructor function.
   *
   * In case of a closure, it can be recreated by bringing the needed values
   * into scope and creating an arrow function:
   *
   *     const { paramA, paramB } = (blob as any).some.property
   *     return { myFunc: (x, y) => x * paramA + y * paramB }
   */
  deserialize: (blob: Serialized) => S
}

export const SnapshotFormat = {
  identity: <S>(version: number): SnapshotFormat<S, S> => ({
    version,
    serialize: x => x,
    deserialize: x => x,
  }),
}

export type StateEffect<C, P> = SendSelfCommand<C> | PublishState<P>

export type StateSubscription<S, C, P> = {
  /**
   * A state subscription is identified by its name. Two state subscriptions with the same name
   * are considered identical
   */
  readonly name: string
  /**
   * Create an observable based on observing the state of any number of fishes.
   * It is the responsibility of the pond to actually subscribe to the returned observable and to
   * apply the returned effects. The fish should not do so itself.
   */
  readonly create: OnStateChange<S, C, P>
}

/**
 * A fish type is described by its semantic name and three functions, responding
 * to the initial creation, to commands, and to events, respectively.
 *
 * The only implementor of this is FishTypeImpl. Implementing this type in any
 * other way will not work.
 */
export interface FishType<C, E, P> {
  command: C
  event: E
  state: P
  semantics: Semantics
}

export type FishConfig<S, C, E, P> = {
  semantics: Semantics
  initialState: InitialState<S>
  onEvent?: OnEvent<S, E>
  onCommand?: OnCommand<S, C, E>
  onStateChange?: OnStateChange<S, C, P>
  semanticSnapshot?: SemanticSnapshot<E>
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  localSnapshot?: SnapshotFormat<S, any>
}

const mkFishType = <S, C, E, P>(config: FishConfig<S, C, E, P>): FishTypeImpl<S, C, E, P> => {
  const noopOnEvent: OnEvent<S, E> = (state: S) => state
  const noopOnCommand: OnCommand<S, C, E> = () => []
  const noopOnStateChange: OnStateChange<S, C, P> = () => Observable.never()
  const onEvent = config.onEvent || noopOnEvent
  const onCommand = config.onCommand || noopOnCommand
  const onStateChange = config.onStateChange || noopOnStateChange
  return new FishTypeImpl<S, C, E, P>(
    config.semantics,
    config.initialState,
    onEvent,
    onCommand,
    onStateChange,
    config.semanticSnapshot,
    config.localSnapshot,
  )
}

export class FishTypeImpl<S, C, E, P> implements FishType<C, E, P> {
  static of = mkFishType

  // see comment in FishType above
  // eslint-disable-next-line @typescript-eslint/ban-ts-ignore
  // @ts-ignore
  command: C
  // eslint-disable-next-line @typescript-eslint/ban-ts-ignore
  // @ts-ignore
  event: E
  // eslint-disable-next-line @typescript-eslint/ban-ts-ignore
  // @ts-ignore
  state: P
  // eslint-disable-next-line @typescript-eslint/ban-ts-ignore
  // @ts-ignore
  privateState: S

  constructor(
    readonly semantics: Semantics,
    readonly initialState: InitialState<S>,
    readonly onEvent: OnEvent<S, E>,
    readonly onCommand: OnCommand<S, C, E>,
    readonly onStateChange: OnStateChange<S, C, P>,
    readonly semanticSnapshot: SemanticSnapshot<E> | undefined,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    readonly localSnapshot: SnapshotFormat<S, any> | undefined,
  ) {}

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  static downcast<C, E, P>(f: FishType<C, E, P>): FishTypeImpl<any, C, E, P> {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    return f as any
  }
}

export type ObserveMethod = <C, E, P>(fish: FishType<C, E, P>, fishName: string) => Observable<P>

export type PondObservables<Self> = {
  /**
   * Obtain an observable stream of states from the given fish, waking it up if it is
   * not already actively running within this pond. It is guaranteed that after a
   * change in state there will eventually be a current state object emitted by the
   * returned observable, but not every intermediate state is guaranteed to be emitted.
   */
  observe: ObserveMethod

  /**
   * Observe the private state update stream of the fish itself.
   */
  observeSelf: () => Observable<Self>
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function mkSource(semantics: FishType<any, any, any>, name: FishName, sourceId: SourceId): Source {
  return { semantics: semantics.semantics, name, sourceId }
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function mkTarget<C, C1 extends C>(semantics: FishType<C, any, any>, name: FishName): Target<C1> {
  return {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    semantics: semantics as FishType<C1, any, any>,
    name,
  }
}

export const Source = {
  of: mkSource,
  format: streamName,
}

export const Target = {
  of: mkTarget,
}

export const StateEffect = {
  sendSelf: mkSendSelf,
  publish: mkPublish,
}

export const FishType = {
  of: mkFishType,
}

export const StateSubscription = {
  publishState,
}

export type MixedDefined = object | number | string | boolean

export const enum ValidationFailure {
  InvalidPayload = 'InvalidPayload',
  InvalidCommand = 'InvalidCommand',
}
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type CommandValidator<T> = (command: any) => Either<ValidationFailure, T>

export const SendCommand = {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  of: <C>(semantics: FishType<C, any, any>, name: FishName, command: C): SendCommand<C> => ({
    target: Target.of(semantics, name),
    command,
  }),
}

/**
 * A state and its corresponding psn map
 */
export type StateWithProvenance<S> = {
  readonly state: S
  /**
   * Minimum psn map that allow to reconstruct the state.
   * Only contains sources that contain events matching the filter.
   */
  readonly psnMap: OffsetMap
}

export type LocalSnapshot<S> = StateWithProvenance<S> & {
  /**
   * eventKey of the last event according to event order that went into the state.
   * This can be used to detect shattering of the state due to time travel.
   */
  eventKey: EventKey

  /**
   * Oldest event key we are interested in. This is defined for a local snapshot
   * that is based on a semantic snapshot. All events before the semantic snapshot
   * that the local snapshot is based on are not relevant and can be discarded.
   *
   * Not discarding these events will lead to unnecessary shattering.
   */
  horizon: EventKey | undefined

  /**
   * Number of events since the beginning of time or the last semantic snapshot (which is
   * kind of the same thing as far as the fish is concerned). This can be used as a measure
   * how useful the snapshot is, and also for count-based snapshot scheduling
   */
  cycle: number
}

export type TaggedIndex = {
  // The index of some array, that we have tagged.
  // It’s mutable because StatePointer<S, E> is meant to be updated when the referenced array changes.
  i: number
  readonly tag: string
  readonly persistAsLocalSnapshot: boolean
}

export const TaggedIndex = {
  ord: contramap((ti: TaggedIndex) => ti.i, ordNumber),
}

export type CachedState<S> = {
  readonly state: StateWithProvenance<S>
  readonly finalIncludedEvent: Event
}

export type StatePointer<S> = TaggedIndex & CachedState<S>
