import { TagIntersection, TagUnion } from './untyped'

export type Tag<E> = Readonly<{
  // raw tag
  tag: string

  // underlying data type guaranteed by user
  _dataType?: E
}>

const subTags = <E>(tag: Tag<E>, ...path: string[]): Tag<E>[] => {
  let curr = tag.tag
  return path.reduce(
    (tags, element) => {
      curr = curr + ':' + element
      tags.push({ tag: curr })
      return tags
    },
    [tag],
  )
}

export const Tag = {
  mk: <E>(rawTag: string): Tag<E> => ({ tag: rawTag } as Tag<E>),

  subTags,
}

const extractTagStrings = (tags: ReadonlyArray<Tag<unknown>>) => tags.map(x => x.tag)

export class EmissionTags<E> {
  private tags: ReadonlyArray<string> = []

  add<E1>(...tags: Tag<E>[]): EmissionTags<Extract<E, E1>> {
    const r = new EmissionTags<unknown>()
    r.tags = this.tags.concat(extractTagStrings(tags))
    return r as EmissionTags<Extract<E, E1>>
  }

  addPath<E1>(tag: Tag<E>, ...path: string[]): EmissionTags<Extract<E, E1>> {
    const tags = Tag.subTags(tag, ...path)
    return this.add(...tags)
  }

  raw(): ReadonlyArray<string> {
    return this.tags
  }
}

export type TypedTagUnion<E> = Readonly<{
  _dataType?: E

  raw(): TagUnion

  type: 'typed-union'
}>

export type TypedTagIntersection<E> = Readonly<{
  and<E1>(tag: Tag<E1>): TypedTagIntersection<Extract<E1, E>>

  andPath<E1>(tag: Tag<E1>, ...path: string[]): TypedTagIntersection<Extract<E1, E>>

  raw(): TagIntersection

  _dataType?: E

  type: 'typed-intersection'
}>

export type TypedTagQuery<E> = TypedTagUnion<E> | TypedTagIntersection<E>

const req = <E>(onlyLocalEvents: boolean, tags: Tag<E>[]): TypedTagIntersection<E> => {
  return {
    and: <E1>(tag: Tag<E1>) => {
      const cast = [...tags, tag] as Tag<Extract<E1, E>>[]
      return req(onlyLocalEvents, cast)
    },

    andPath: <E1>(tag: Tag<E1>, ...path: string[]) => {
      const moreTags = Tag.subTags(tag, ...path)
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

const matchAnyOf = <E>(...sets: TypedTagIntersection<E>[]): TypedTagUnion<E> => {
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
  requireLocalTag: <E>(x: Tag<E>) => req(true, [x]),

  requirePath: <E>(tag: Tag<E>, ...path: string[]) => req(false, Tag.subTags(tag, ...path)),
  requireLocalPath: <E>(tag: Tag<E>, ...path: string[]) => req(true, Tag.subTags(tag, ...path)),

  matchAnyOf,
}
