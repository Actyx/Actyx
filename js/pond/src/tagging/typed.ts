import { TagIntersection, TagUnion } from './untyped'

export type Tag<E> = Readonly<{
  // raw tag
  tag: string

  // underlying data type guaranteed by user
  _dataType?: E
}>

export const Tag = {
  mk: <E>(rawTag: string): Tag<E> => ({ tag: rawTag } as Tag<E>),
}

const extractTagStrings = (tags: ReadonlyArray<Tag<unknown>>) => tags.map(x => x.tag)

export const namedSubTags = <E>(tag: Tag<E>, ...path: string[]): Tag<E>[] => {
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

export class EmissionTags<E> {
  private tags: ReadonlyArray<string> = []

  add<E1>(...tags: Tag<E>[]): EmissionTags<Extract<E, E1>> {
    const r = new EmissionTags<unknown>()
    r.tags = this.tags.concat(extractTagStrings(tags))
    return r as EmissionTags<Extract<E, E1>>
  }

  addPath<E1>(tag: Tag<E>, ...path: string[]): EmissionTags<Extract<E, E1>> {
    const tags = namedSubTags(tag, ...path)
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
  and<E1>(...tags: Tag<E1>[]): TypedTagIntersection<Extract<E1, E>>

  raw(): TagIntersection

  _dataType?: E

  type: 'typed-intersection'
}>

export type TypedTagQuery<E> = TypedTagUnion<E> | TypedTagIntersection<E>

const req = (onlyLocalEvents: boolean) => <E>(...tags: Tag<E>[]): TypedTagIntersection<E> => {
  return {
    and: <E1>(...moreTags: Tag<E1>[]) => {
      const cast = [...tags, ...moreTags] as Tag<Extract<E1, E>>[]
      return req(onlyLocalEvents)<Extract<E1, E>>(...cast)
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
  requireTag: req(false),
  requireLocalTag: req(true),

  matchAnyOf,
}
