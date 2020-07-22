import { TagIntersection, TagUnion } from './untyped'
import { isString } from '..'

const namedSubSpace = (rawTag: string, sub: string): string[] => {
  return [rawTag, rawTag + ':' + sub]
}

export interface TypedTagUnion<E> {
  /**
   * Add an alternative set we may also match. E.g. tag0.or(tag1.and(tag2)).or(tag1.and(tag3)) will match:
   * Events with tag0; Events with both tag1 and tag2; Events with both tag1 and tag3.
   */
  or<E1>(tag: Tags<E1>): TypedTagUnion<E1 | E>

  /**
   * Convert into an untyped TagQuery. This is for internal use.
   */
  raw(): TagUnion

  /**
   * Aggregated type of the Events which may be returned by the contained tags.
   */
  readonly _dataType?: E

  readonly type: 'typed-union'
}

// Must be interface, otherwise inferred (recursive) type gets very large.
export interface Tags<E> {
  /**
   * Add another tag(s) to this requirement. E.g Tag<FooEvent>('foo').and(Tag<BarEvent>('bar')) will require both 'foo' and 'bar'.
   */
  and<E1>(tag: Tags<E1>): Tags<Extract<E1, E>>

  /**
   * Add an additional untyped tag to this requirement.
   * Since there is no associated type, the overall type cannot be constrained further.
   */
  and(tag: string): Tags<E>

  /**
   * Add an alternative set we may also match. E.g. Tag<FooEvent>('foo').or(Tag<BarEvent>('bar')) will match
   * each Event with at least 'foo' or 'bar'. Note that after the first `or` invocation you cannot `and` anymore,
   * so you have to nest the parts yourself: tag0.or(tag1.and(tag2)).or(tag1.and(tag3)) etc.
   */
  or<E1>(tag: Tags<E1>): TypedTagUnion<E1 | E>

  /**
   * The same requirement, but matching only Events emitted by the very node the code is run on.
   */
  local(): Tags<E>

  /**
   * Convert into an untyped TagQuery. This is for internal use.
   */
  raw(): TagIntersection

  /**
   * Aggregated type of the Events which may be returned by the contained tags.
   */
  readonly _dataType?: E

  readonly type: 'typed-intersection'
}

export const Tags = <E>(...requiredTags: string[]): Tags<E> => req<E>(false, requiredTags)

export interface Tag<E> extends Tags<E> {
  // The underlying actual tag as pure string
  readonly rawTag: string

  withId(name: string): Tags<E>
}

export const Tag = <E>(rawTag: string): Tag<E> => ({
  rawTag,

  withId: (name: string) => req(false, namedSubSpace(rawTag, name)),

  ...req(false, [rawTag]),
})

/**
 * Typed expression for tag statements. The type `E` describes which events may be annotated with the included tags.
 */
export type Where<E> = TypedTagUnion<E> | Tags<E>

const req = <E>(onlyLocalEvents: boolean, rawTags: string[]): Tags<E> => {
  const r: Tags<E> = {
    and: <E1>(tag: Tags<E1> | string) => {
      if (isString(tag)) {
        return req<E>(onlyLocalEvents, [tag, ...rawTags])
      }

      const other = tag.raw()

      const local = onlyLocalEvents || !!other.onlyLocalEvents
      const tags = rawTags.concat(other.tags)

      return req<Extract<E1, E>>(local, tags)
    },

    or: <E1>(other: Tags<E1>) => {
      return union<E1 | E>([req(onlyLocalEvents, rawTags), other])
    },

    local: () => req<E>(true, rawTags),

    type: 'typed-intersection',

    raw: () => ({
      type: 'intersection',

      tags: rawTags,

      onlyLocalEvents,
    }),
  }

  return r
}

const union = <E>(sets: Tags<unknown>[]): TypedTagUnion<E> => {
  return {
    type: 'typed-union',

    or: <E1>(other: Tags<E1>) => {
      return union<E1 | E>([...sets, other])
    },

    raw: () => ({
      type: 'union',
      tags: sets.map(x => x.raw()),
    }),
  }
}

