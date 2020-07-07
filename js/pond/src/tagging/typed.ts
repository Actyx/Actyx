import { TagIntersection, TagUnion } from './untyped'

export type Tag<E> = Readonly<{
  // raw tag
  tag: string

  // underlying data type guaranteed by user
  _dataType?: E
}>

const namedSubSpace = <E>(tag: Tag<E>, name: string): Tag<E>[] => {
  return [tag, { tag: tag.tag + ':' + name }]
}

export const Tag = {
  mk: <E>(rawTag: string): Tag<E> => ({ tag: rawTag } as Tag<E>),

  namedSubSpace,
}

const extractTagStrings = (tags: ReadonlyArray<Tag<unknown>>) => tags.map(x => x.tag)

export class EmissionTags<E> {
  private tags: ReadonlyArray<string> = []

  add<E1>(...tags: Tag<E>[]): EmissionTags<Extract<E, E1>> {
    const r = new EmissionTags<unknown>()
    r.tags = this.tags.concat(extractTagStrings(tags))
    return r as EmissionTags<Extract<E, E1>>
  }

  addNamed<E1>(tag: Tag<E>, name: string): EmissionTags<Extract<E, E1>> {
    const tags = Tag.namedSubSpace(tag, name)
    return this.add(...tags)
  }

  raw(): ReadonlyArray<string> {
    return this.tags
  }
}

export interface TypedTagUnion<E> {
  raw(): TagUnion

  readonly _dataType?: E

  readonly type: 'typed-union'
}

// Must be interface, otherwise inferred (recursive) type gets very large.
export interface TypedTagIntersection<E> {
  and<E1>(tag: Tag<E1>): TypedTagIntersection<Extract<E1, E>>

  andNamed<E1>(tag: Tag<E1>, name: string): TypedTagIntersection<Extract<E1, E>>

  raw(): TagIntersection

  readonly _dataType?: E

  readonly type: 'typed-intersection'
}

export type TypedTagQuery<E> = TypedTagUnion<E> | TypedTagIntersection<E>

const req = <E>(onlyLocalEvents: boolean, tags: Tag<E>[]): TypedTagIntersection<E> => {
  return {
    and: <E1>(tag: Tag<E1>) => {
      const cast = [...tags, tag] as Tag<Extract<E1, E>>[]
      return req(onlyLocalEvents, cast)
    },

    andNamed: <E1>(tag: Tag<E1>, name: string) => {
      const moreTags = Tag.namedSubSpace(tag, name)
      const cast = [...tags, ...moreTags] as Tag<Extract<E1, E>>[]
      return req(onlyLocalEvents, cast)
    },

    type: 'typed-intersection',

    raw: () => ({
      type: 'intersection',

      tags: extractTagStrings(tags),

      onlyLocalEvents,
    }),
  }
}

export const matchAnyOf = <E>(...sets: TypedTagIntersection<E>[]): TypedTagUnion<E> => {
  return {
    type: 'typed-union',

    raw: () => ({
      type: 'union',
      tags: sets.map(x => x.raw()),
    }),
  }
}

export const TypedTagQuery = {
  requireTag: <E>(x: Tag<E>) => req(false, [x]),
  requireNamed: <E>(tag: Tag<E>, name: string) => req(false, Tag.namedSubSpace(tag, name)),

  requireLocalTag: <E>(x: Tag<E>) => req(true, [x]),
  requireLocalNamed: <E>(tag: Tag<E>, name: string) => req(true, Tag.namedSubSpace(tag, name)),

  matchAnyOf,
}
