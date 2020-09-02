import { isString } from '..'
import { TagSubscription } from '../subscription'

const namedSubSpace = (rawTag: string, sub: string): string[] => {
  return [rawTag, rawTag + ':' + sub]
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
   * Can be used to uniquely identify a set of events.
   */
  toString(): string

  /**
   * FOR INTERNAL USE. Convert to Actyx wire format.
   * @internal
   */
  toWireFormat(): ReadonlyArray<TagSubscription>

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
   * Add more tags to this requirement. E.g Tag<FooEvent>('foo').and(Tag<BarEvent>('bar')) will require both 'foo' and 'bar'.
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
   * The actual included tags.
   * @internal
   */
  readonly rawTags: ReadonlyArray<string>

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
export const Tags = <E>(...requiredTags: string[]): Tags<E> => req<E>(false, requiredTags)

/**
 * Representation of a single tag.
 * @public
 */
export interface Tag<E> extends Tags<E> {
  // The underlying actual tag as pure string.
  readonly rawTag: string

  /**
   * This very tag, suffixed with an id. E.g. `Tag<RobotEvent>('robot').withId('robot500')`
   * expresses robot events belonging to a *specific* robot. The suffix will be separated
   * from the base name by a colon `:`.
   */
  withId(name: string): Tags<E>
}

/**
 * Create a new tag from the given string.
 * (Tag factory function. Call WITHOUT new, e.g. `const myTag = Tag<MyType>('my-tag')`)
 * @public
 */
export const Tag = <E>(rawTag: string): Tag<E> => ({
  rawTag,

  withId: (name: string) => req(false, namedSubSpace(rawTag, name)),

  ...req(false, [rawTag]),
})

const req = <E>(onlyLocalEvents: boolean, rawTags: string[]): Tags<E> => {
  const r: Tags<E> = {
    and: <E1>(otherTags: Tags<E1> | string) => {
      if (isString(otherTags)) {
        return req<E>(onlyLocalEvents, [otherTags, ...rawTags])
      }

      const local = onlyLocalEvents || !!otherTags.onlyLocalEvents
      const tags = rawTags.concat(otherTags.rawTags)

      return req<Extract<E1, E>>(local, tags)
    },

    or: <E1>(other: Where<E1>) => {
      return other.merge<E | E1>([r])
    },

    local: () => req<E>(true, rawTags),

    onlyLocalEvents,

    rawTags,

    toWireFormat: () => [{ local: onlyLocalEvents, tags: rawTags }],

    merge: <T>(moreSets: Tags<unknown>[]) => union<T>(moreSets.concat(r)),

    toString: () => {
      if (rawTags.length === 0) {
        return 'allEvents'
      }

      return (
        rawTags
          .sort()
          .map(escapeTag)
          .join(' & ') + (onlyLocalEvents ? ' & isLocal' : '')
      )
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

    toWireFormat: () => sets.map(x => ({ local: x.onlyLocalEvents, tags: x.rawTags })),

    toString: () =>
      sets
        .map(s => s.toString())
        .sort()
        .join(' | '),
  }
}

/**
 * A `Where` expression that selects all events.
 * @public
 */
export const allEvents: Tags<unknown> = req(false, [])

/**
 * A `Where` expression that selects no events.
 * @public
 */
export const noEvents: Where<never> = union([])

/** @internal */
export const escapeTag = (rawTag: string) => "'" + rawTag.replace(/'/g, "''") + "'"
