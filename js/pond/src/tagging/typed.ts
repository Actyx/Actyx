import { TagIntersection, TagUnion } from './untyped'
import { isString } from '..'

const namedSubSpace = (rawTag: string, sub: string): string[] => {
  return [rawTag, rawTag + ':' + sub]
}

// "TagUnion"
type TagSets = Readonly<{

  orU(tags: string | Tags | TypedTagIntersection<unknown>): TagSets

  type: 'untyped-union'


  /**
   * Convert into an untyped TagQuery. This is for internal use.
   */
  raw(): TagUnion
}>

// "TagIntersection"
type Tags = Readonly<{
  and<E1>(tag: TypedTagIntersection<E1>): TypedTagIntersection<E1>

  and(tag: string | Tags): Tags

  or<E1>(tags: TypedTagIntersection<E1> | string | Tags): TagSets

  orU(tags: string): TagSets

  /**
   * Add an alternative untyped tag we may also match.
   * Since the tag is untyped, the events included in the resulting set may have any type.
   */
  or(tag: string): TagSets

  /**
   * The same requirement, but matching only Events emitted by the very node the code is run on.
   */
  local(): Tags


  /**
   * Convert into an untyped TagQuery. This is for internal use.
   */
  raw(): TagIntersection

  type: 'untyped-intersection'
}>

const TagsImpl = (onlyLocalEvents: boolean, rawTags: string[]): Tags => {

  const r: Tags = {
    and: <E1>(tag: Tags | TypedTagIntersection<E1> | string) => {
      if (isString(tag)) {
        const r0: Tags = TagsImpl(onlyLocalEvents, [tag, ...rawTags])
        return r0
      }

      if (tag.type === 'typed-intersection') {
        const r1: TypedTagIntersection<E1> = tag.and(req<any>(onlyLocalEvents, rawTags))
        return r1
      }

      const other: TagIntersection = tag.raw()

      const local = onlyLocalEvents || !!other.onlyLocalEvents
      const tags = rawTags.concat(other.tags)

      const r2: Tags = TagsImpl(local, tags)
      return r2
    },


    orU: (other: string) => {
      return TagSets([req(onlyLocalEvents, rawTags).raw(), Tag(other).raw()])
    },

    or: <E1>(other: TypedTagIntersection<E1> | string | Tags) => {
      const otherRaw = isString(other) ? Tag(other).raw() : other.raw()

      return TagSets([r.raw(), otherRaw])
    },

    local: () => TagsImpl(true, rawTags),

    type: 'untyped-intersection',

    raw: () => ({
      type: 'intersection',

      tags: rawTags,

      onlyLocalEvents,
    }),
  }

  return r
}

export const Tags = (...requiredTags: string[]): Tags => TagsImpl(false, requiredTags)
export const LocalTags = (...requiredTags: string[]): Tags => TagsImpl(true, requiredTags)

const TagSets = (sets: TagIntersection[]): TagSets => {
  return {
    type: 'untyped-union',

    orU: <E1>(other: TypedTagIntersection<E1> | string | Tags) => {
      const otherRaw = isString(other) ? Tag(other).raw() : other.raw()

      return TagSets([...sets, otherRaw])
    },

    raw: () => ({
      type: 'union',
      tags: sets,
    }),
  }
}





export interface TypedTagUnion<E> {
  /**
   * Add an alternative untyped tag we may also match.
   * Since the tag is untyped, the events included in the resulting set may have any type.
   */
  orU(tag: string): TagSets

  /**
   * Add an alternative set we may also match. E.g. tag0.or(tag1.and(tag2)).or(tag1.and(tag3)) will match:
   * Events with tag0; Events with both tag1 and tag2; Events with both tag1 and tag3.
   */
  or<E1>(tag: TypedTagIntersection<E1>): TypedTagUnion<E1 | E>

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
export interface TypedTagIntersection<E> {
  /**
   * Add another tag(s) to this requirement. E.g Tag<FooEvent>('foo').and(Tag<BarEvent>('bar')) will require both 'foo' and 'bar'.
   */
  and<E1>(tag: TypedTagIntersection<E1>): TypedTagIntersection<Extract<E1, E>>

  /**
   * Add an additional untyped tag to this requirement.
   * Since there is no associated type, the overall type cannot be constrained further.
   */
  and(tag: string): TypedTagIntersection<E>

  /**
   * Add an alternative set we may also match. E.g. Tag<FooEvent>('foo').or(Tag<BarEvent>('bar')) will match
   * each Event with at least 'foo' or 'bar'. Note that after the first `or` invocation you cannot `and` anymore,
   * so you have to nest the parts yourself: tag0.or(tag1.and(tag2)).or(tag1.and(tag3)) etc.
   */
  or<E1>(tag: TypedTagIntersection<E1>): TypedTagUnion<E1 | E>

  /**
   * Add an alternative untyped tag we may also match.
   * Since the tag is untyped, the events included in the resulting set may have any type.
   */
  orU(tag: string): TagSets

  /**
   * The same requirement, but matching only Events emitted by the very node the code is run on.
   */
  local(): TypedTagIntersection<E>

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
 * Typed expression for tag statements. The type `E` describes which events may be annotated with the included tags.
 */
export type Where<E> = TypedTagUnion<E> | TypedTagIntersection<E>

const req = <E>(onlyLocalEvents: boolean, rawTags: string[]): TypedTagIntersection<E> => {
  const r: TypedTagIntersection<E> = {
    and: <E1>(tag: TypedTagIntersection<E1> | string) => {
      if (isString(tag)) {
        return req<E>(onlyLocalEvents, [tag, ...rawTags])
      }

      const other = tag.raw()

      const local = onlyLocalEvents || !!other.onlyLocalEvents
      const tags = rawTags.concat(other.tags)

      return req<Extract<E1, E>>(local, tags)
    },

    orU: (other: string) => {
      return TagSets([req(onlyLocalEvents, rawTags).raw(), Tag(other).raw()])
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

    orU: (other: string) => {
      return TagSets([Tag(other).raw(), ...sets.map(x => x.raw())])

    },

    raw: () => ({
      type: 'union',
      tags: sets.map(x => x.raw()),
    }),
  }
}

