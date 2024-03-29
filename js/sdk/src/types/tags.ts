/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { noop } from '../util'
import { isString } from './functions'
import { TaggedEvent, TaggedTypedEvent } from './various'

/** V1 tag subscription wire format. @internal */
export type TagSubscription = { tags: string[]; local: boolean }

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

const makeInternal = (tag: string): TagInternal => {
  if (tag === '') {
    throw new Error('Tag cannot be empty string')
  }
  return {
    tag,
    extractId: noop,
  }
}

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
  toV1WireFormat(): TagSubscription[]

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
export interface Tags<E = unknown> extends Where<E> {
  /**
   * Add more tags to this requirement. E.g `Tag<FooEvent>('foo').and(Tag<BarEvent>('bar'))` will require both 'foo' and 'bar'.
   */
  and<E1 = unknown>(tag: Tags<E1>): Tags<E1 & E>

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
  apply(...events: E[]): TaggedEvent[]

  /**
   * Apply these tags to a list of events that match their type, and allow further tags to be added.
   */
  applyTyped<E1 extends E>(event: E1): TaggedTypedEvent<E1>

  /**
   * The actual included tags. @internal
   */
  readonly rawTags: TagInternal[]

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
export const Tags = <E = unknown>(...requiredTags: string[]): Tags<E> =>
  req<E>(false, requiredTags.map(makeInternal))

/**
 * Representation of a single tag.
 * @public
 */
export interface Tag<E = unknown> extends Tags<E> {
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

  /**
   * Returns the ID-specific variant of this particular tag, see also `withId`.
   *
   * Use this if you want to tag an event belonging to entity A with a specific identity of entity B.
   * For example an activity has been started by some user, so you may add
   * `activityTag.withId(activityId).and(userTag.id(userId))`; this would result in three tags.
   *
   * This returns a tag for arbitrary events (type `unknown`) because the base tag is omitted.
   */
  id(name: string): Tags<unknown>
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
export const Tag = <E = unknown>(rawTagString: string, extractId?: (e: E) => string): Tag<E> => {
  const internalTag: TagInternal = {
    ...makeInternal(rawTagString),
    extractId: extractId || noop,
  }

  const withId = (name: string): Tags<E> =>
    req(false, namedSubSpace(rawTagString, name).map(makeInternal))
  const id = (name: string): Tags<unknown> =>
    req(false, [{ tag: `${rawTagString}:${name}`, extractId: noop }])

  return {
    rawTag: rawTagString,
    withId,
    id,
    ...req(false, [internalTag]),
  }
}

const req = <E>(onlyLocalEvents: boolean, rawTags: TagInternal[]): Tags<E> => {
  const r: Tags<E> = {
    and: <E1>(otherTags: Tags<E1> | string) => {
      if (isString(otherTags)) {
        return req<E>(onlyLocalEvents, [makeInternal(otherTags), ...rawTags])
      }

      const local = onlyLocalEvents || !!otherTags.onlyLocalEvents
      const tags = rawTags.concat(otherTags.rawTags)

      return req<E1 & E>(local, tags)
    },

    or: <E1>(other: Where<E1>) => {
      return other.merge<E | E1>([r])
    },

    local: () => req<E>(true, rawTags),

    apply: (...events: E[]) => {
      if (events.length === 1) {
        const event = events[0]
        const res: TaggedEvent = { event, tags: rawTags.flatMap(autoExtract(event)) }
        // TS cannot fathom our awesome "if arg list length is 1, there is not a list of args but just a single arg" logic
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        return res as any
      } else {
        const res: TaggedEvent[] = events.map((event) => ({
          event,
          tags: rawTags.flatMap(autoExtract(event)),
        }))
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        return res as any
      }
    },

    applyTyped: <E1>(event: E1) => {
      const withTags =
        (myTags: readonly TagInternal[]) =>
        <E2>(tags: Tags<unknown> | (Tags<E2> & (E extends E2 ? unknown : never))) => {
          const myTags2 = [...myTags, ...tags.rawTags]
          return { event, tags: myTags2.map((x) => x.tag), withTags: withTags(myTags2) }
        }
      return withTags(
        rawTags.flatMap(autoExtract(event)).map((tag) => ({ tag, extractId: () => undefined })),
      )(Tags())
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
