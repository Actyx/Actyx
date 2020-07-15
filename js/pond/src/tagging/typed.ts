import { TagIntersection, TagUnion } from './untyped'

const namedSubSpace = (rawTag: string, sub: string): string[] => {
  return [rawTag, rawTag + ':' + sub]
}

export interface TypedTagUnion<E> {
  /**
   * Add an alternative set we may also match. E.g. tag0.or(tag1.and(tag2)).or(tag1.and(tag3)) will match:
   * Events with tag0; Events with both tag1 and tag2; Events with both tag1 and tag3.
   *
   * @param ..
   * @returns       ..
   */
  or<E1>(tag: TypedTagIntersection<E1>): TypedTagUnion<E1 | E>

  /**
   * Convert into an untyped TagQuery. This is for internal use.
   *
   * @param ..
   * @returns       ..
   */
  raw(): TagUnion

  /**
   * Aggregated type of the Events which may be returned by the contained tags.
   */
  readonly _dataType?: E

  readonly type: 'typed-union'
}

// Must be interface, otherwise inferred (recursive) type gets very large.
export interface TypedTagIntersection<E> {
  /**
   * Add another tag(s) to this requirement. E.g Tag<FooEvent>('foo').and(Tag<BarEvent>('bar')) will require both 'foo' and 'bar'.
   *
   * @param ..
   * @returns       ..
   */
  and<E1>(tag: TypedTagIntersection<E1>): TypedTagIntersection<Extract<E1, E>>

  /**
   * Add an alternative set we may also match. E.g. Tag<FooEvent>('foo').or(Tag<BarEvent>('bar')) will match 
   * each Event with at least 'foo' or 'bar'. Note that after the first `or` invocation you cannot `and` anymore,
   * so you have to nest the parts yourself: tag0.or(tag1.and(tag2)).or(tag1.and(tag3)) etc.
   *
   * @param ..
   * @returns       ..
   */
  or<E1>(tag: TypedTagIntersection<E1>): TypedTagUnion<E1 | E>


  /**
   * The same requirement, but matching only Events emitted by the very node the code is run on.
   *
   * @param ..
   * @returns       ..
   */
  local(): TypedTagIntersection<E>


  /**
   * Convert into an untyped TagQuery. This is for internal use.
   *
   * @param ..
   * @returns       ..
   */
  raw(): TagIntersection

  /**
   * Aggregated type of the Events which may be returned by the contained tags.
   */
  readonly _dataType?: E

  readonly type: 'typed-intersection'
}

export interface Tag<E> extends TypedTagIntersection<E> {
  // The underlying actual tag as pure string
  readonly rawTag: string

  withId(name: string): TypedTagIntersection<E>
}

export const Tag = <E>(rawTag: string): Tag<E> => ({
  rawTag,

  withId: (name: string) => req(false, namedSubSpace(rawTag, name)),

  ...req(false, [rawTag]),
})

/**
 * Typed expression for tag statements.
 */
export type Where<E> = TypedTagUnion<E> | TypedTagIntersection<E>

const req = <E>(onlyLocalEvents: boolean, rawTags: string[]): TypedTagIntersection<E> => {
  const r: TypedTagIntersection<E> = {
    and: <E1>(tag: TypedTagIntersection<E1>) => {
      const other = tag.raw()

      const local = onlyLocalEvents || !!other.onlyLocalEvents
      const tags = rawTags.concat(other.tags)

      return req<Extract<E1, E>>(local, tags)
    },

    or: <E1>(other: TypedTagIntersection<E1>) => {
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

const union = <E>(sets: TypedTagIntersection<unknown>[]): TypedTagUnion<E> => {
  return {
    type: 'typed-union',

    or: <E1>(other: TypedTagIntersection<E1>) => {
      return union<E1 | E>([...sets, other])
    },

    raw: () => ({
      type: 'union',
      tags: sets.map(x => x.raw()),
    }),
  }
}
