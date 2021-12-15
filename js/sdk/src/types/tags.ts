/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { noop } from '../util'
import { isString } from './functions'
import { TaggedEvent } from './various'

/** V1 tag subscription wire format. @internal */
export type TagSubscription = Readonly<{ tags: ReadonlyArray<string>; local: boolean }>

const namedSubSpace = (rawTag: string, sub: string): string[] => {
  return [rawTag, rawTag + ':' + sub]
}

/**
 * An internal raw tag can either be a plain string, or a function that produces tags from an event.
 * @internal */
type TagInternal = {
  tag: string

  /** Using `any` here since `unknown` leads to trouble in usage. */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  extractId: (event: any) => string | undefined
}

const makeInternal = (tag: string): TagInternal => ({
  tag,
  extractId: noop,
})

const justTag = (i: TagInternal) => i.tag

// Try automatically extracting the "id" for a given event from a tag
/** @returns List of tags as strings to be applied to the event (1 or 2 elements). */
const autoExtract =
  (event: unknown) =>
  (t: TagInternal): string[] => {
    const id = t.extractId(event)
    if (id) {
      return namedSubSpace(t.tag, id)
    }

    return [t.tag]
  }

/**
 * Representation of a union of tag sets. I.e. this is an event selection that combines multiple `Tags` selections.
 * @public
 */
export interface Where<E> {
  /**
   * Add an alternative set we may also match. E.g. tag0.or(tag1.and(tag2)).or(tag1.and(tag3)) will match:
   * Events with tag0; Events with both tag1 and tag2; Events with both tag1 and tag3.
   */
  or<E1>(tag: Where<E1>): Where<E1 | E>

  /**
   * Type of the Events which may be returned by the contained tags.
   * Note that this does reflect only locally declared type knowledge;
   * historic events delivered by the Actyx system may not match these types, and this is not automatically detected.
   * It is therefore good practice to carefully review changes to the declared type so that they remain
   * backwards compatible.
   */
  readonly _dataType?: E

  /**
   * Convert to an Actyx Event Service query string.
   */
  toString(): string

  /**
   * FOR INTERNAL USE. Convert to Actyx wire format.
   * @internal
   */
  toV1WireFormat(): ReadonlyArray<TagSubscription>

  /**
   * For merging with another Where statement. (Worse API than the public one, but easier to implement.)
   * @internal
   */
  merge<T>(tagsSets: Tags<unknown>[]): Where<T>
}

// Implementation note: We must use interfaces, otherwise inferred (recursive) types get very large.

/**
 * Selection of events based on required tags. `Tags('a', 'b')` will select all events that have tag 'a' *as well as* tag 'b'.
 * @public
 */
export interface Tags<E> extends Where<E> {
  /**
   * Add more tags to this requirement. E.g `Tag<FooEvent>('foo').and(Tag<BarEvent>('bar'))` will require both 'foo' and 'bar'.
   */
  and<E1>(tag: Tags<E1>): Tags<Extract<E1, E>>

  /**
   * Add an additional untyped tag to this requirement.
   * Since there is no associated type, the overall type cannot be constrained further.
   */
  and(tag: string): Tags<E>

  /**
   * The same requirement, but matching only Events emitted by the very node the code is run on.
   * E.g. `Tags('my-tag').local()` selects all locally emitted events tagged with 'my-tag'.
   */
  local(): Tags<E>

  /**
   * Apply these tags to an event that they may legally be attached to (according to the tag's type `E`).
   */
  apply(event: E): TaggedEvent

  /**
   * Apply these tags to a list of events that they may legally be attached to.
   */
  apply(...events: E[]): ReadonlyArray<TaggedEvent>

  /**
   * The actual included tags. @internal
   */
  readonly rawTags: ReadonlyArray<TagInternal>

  /**
   * Whether this specific set is meant to be local-only.
   * @internal
   */
  readonly onlyLocalEvents: boolean
}

/**
 * Declare a set of tags.
 * This is a generator function to be called WITHOUT new, e.g. `const required = Tags('a', 'b', 'c')`
 * @public
 */
export const Tags = <E>(...requiredTags: string[]): Tags<E> =>
  req<E>(false, requiredTags.map(makeInternal))

/**
 * Representation of a single tag.
 * @public
 */
export interface Tag<E> extends Tags<E> {
  /** The underlying actual tag as pure string. @internal */
  readonly rawTag: string

  /**
   * Returns two tags:
   *
   *  - this tag
   *  - this tag suffixed with the given `name`, e.g. `Tag<RobotEvent>('robot').withId('robot500')`
   *    expresses robot events belonging to a *specific* robot. The suffix will be separated
   *    from the base name by a colon `:` like `robot:robot500`.
   *
   * The reason for preserving the base tag is to keep a notion of the whole event group,
   * and enable selection of it all without knowing every individual specific ID.
   */
  withId(name: string): Tags<E>
}

/**
 * Create a new tag from the given string.
 * (Tag factory function. Call WITHOUT new, e.g. `const myTag = Tag<MyType>('my-tag')`)
 *
 * @param rawTagString  - The raw tag string
 * @param extractId     - If supplied, this function will be used to automatically call `withId` for events this tag is being attached to.
 *                        The automatism is disabled if there is a manual `withId` call.
 * @public
 */
export const Tag = <E>(rawTagString: string, extractId?: (e: E) => string): Tag<E> => {
  const internalTag: TagInternal = {
    tag: rawTagString,
    extractId: extractId || noop,
  }

  const withId = (name: string): Tags<E> =>
    req(false, namedSubSpace(rawTagString, name).map(makeInternal))

  return {
    rawTag: rawTagString,

    withId,

    ...req(false, [internalTag]),
  }
}

const req = <E>(onlyLocalEvents: boolean, rawTags: ReadonlyArray<TagInternal>): Tags<E> => {
  const r: Tags<E> = {
    and: <E1>(otherTags: Tags<E1> | string) => {
      if (isString(otherTags)) {
        return req<E>(onlyLocalEvents, [makeInternal(otherTags), ...rawTags])
      }

      const local = onlyLocalEvents || !!otherTags.onlyLocalEvents
      const tags = rawTags.concat(otherTags.rawTags)

      return req<Extract<E1, E>>(local, tags)
    },

    or: <E1>(other: Where<E1>) => {
      return other.merge<E | E1>([r])
    },

    local: () => req<E>(true, rawTags),

    // TS cannot fathom our awesome "if arg list length is 1, there is not a list of args but just a single arg" logic
    /* eslint-disable @typescript-eslint/ban-ts-comment */
    // @ts-ignore
    apply: (...events: E[]) => {
      if (events.length === 1) {
        const event = events[0]
        const res: TaggedEvent = { event, tags: rawTags.flatMap(autoExtract(event)) }
        return res
      } else {
        const res: ReadonlyArray<TaggedEvent> = events.map((event) => ({
          event,
          tags: rawTags.flatMap(autoExtract(event)),
        }))
        return res
      }
    },

    onlyLocalEvents,

    rawTags,

    toV1WireFormat: () => [{ local: onlyLocalEvents, tags: rawTags.map(justTag) }],

    merge: <T>(moreSets: Tags<unknown>[]) => union<T>(moreSets.concat(r)),

    toString: () => {
      if (rawTags.length === 0) {
        return 'allEvents'
      }

      return rawTags.map(escapeTag).join(' & ') + (onlyLocalEvents ? ' & isLocal' : '')
    },
  }

  return r
}

const union = <E>(sets: Tags<unknown>[]): Where<E> => {
  return {
    or: <E1>(other: Where<E1>) => {
      return other.merge<E | E1>(sets)
    },

    merge: <T>(moreSets: Tags<unknown>[]) => union<T>(moreSets.concat(sets)),

    toV1WireFormat: () =>
      sets.map((x) => ({ local: x.onlyLocalEvents, tags: x.rawTags.map(justTag) })),

    toString: () => sets.map((s) => s.toString()).join(' | '),
  }
}

/**
 * A `Where` expression that selects all events.
 * @public
 */
export const allEvents: Tags<unknown> = req(false, [])

/** @internal */
export const escapeTag = (tag: TagInternal) => "'" + tag.tag.replace(/'/g, "''") + "'"
