import { TagIntersection, TagUnion } from './untyped'

const namedSubSpace = (rawTag: string, sub: string): string[] => {
  return [rawTag, rawTag + ':' + sub]
}

export interface TypedTagUnion<E> {
  or<E1>(tag: TypedTagIntersection<E1>): TypedTagUnion<E1 | E>

  raw(): TagUnion

  readonly _dataType?: E

  readonly type: 'typed-union'
}

// Must be interface, otherwise inferred (recursive) type gets very large.
export interface TypedTagIntersection<E> {
  and<E1>(tag: TypedTagIntersection<E1>): TypedTagIntersection<Extract<E1, E>>

  or<E1>(tag: TypedTagIntersection<E1>): TypedTagUnion<E1 | E>

  local(): TypedTagIntersection<E>

  raw(): TagIntersection

  readonly _dataType?: E

  readonly type: 'typed-intersection'
}

export interface Tag<E> extends TypedTagIntersection<E> {
  // The underlying actual tag as pure string
  readonly rawTag: string

  subSpace(name: string): TypedTagIntersection<E>
}

export const Tag = <E>(rawTag: string): Tag<E> => ({
  rawTag,

  subSpace: (name: string) => req(false, namedSubSpace(rawTag, name)),

  ...req(false, [rawTag]),
})

export type TypedTagQuery<E> = TypedTagUnion<E> | TypedTagIntersection<E>

const req = <E>(onlyLocalEvents: boolean, rawTags: string[]): TypedTagIntersection<E> => {
  const r: TypedTagIntersection<E> = {
    and: <E1>(tag: TypedTagIntersection<E1>) => {
      const other = tag.raw()

      const local = onlyLocalEvents || !!other.onlyLocalEvents
      const tags = rawTags.concat(other.tags)

      return req<Extract<E1, E>>(local, tags)
    },

    or: <E1>(other: TypedTagIntersection<E1>) => {
      return matchAnyOf(
        ...([other, req(onlyLocalEvents, rawTags)] as TypedTagIntersection<E1 | E>[]),
      )
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

export const matchAnyOf = <E>(...sets: TypedTagIntersection<E>[]): TypedTagUnion<E> => {
  return {
    type: 'typed-union',

    or: <E1>(other: TypedTagIntersection<E1>) => {
      return matchAnyOf(...([...sets, other] as TypedTagIntersection<E1 | E>[]))
    },

    raw: () => ({
      type: 'union',
      tags: sets.map(x => x.raw()),
    }),
  }
}
