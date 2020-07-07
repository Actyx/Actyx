import { TagIntersection, TagUnion } from './untyped'

const namedSubSpace = (rawTag: string, sub: string): string[] => {
  return [rawTag, rawTag + ':' + sub]
}

export class EmissionTags<E> {
  private tags: ReadonlyArray<string> = []

  private addRaw<E1>(rawTags: string[]): EmissionTags<Extract<E, E1>> {
    const r = new EmissionTags<unknown>()
    r.tags = this.tags.concat(rawTags)
    return r as EmissionTags<Extract<E, E1>>
  }

  add<E1>(...tags: Tag<E>[]): EmissionTags<Extract<E, E1>> {
    return this.addRaw(extractTagStrings(tags))
  }

  addNamed<E1>(tag: Tag<E>, name: string): EmissionTags<Extract<E, E1>> {
    const tags = namedSubSpace(tag.rawTag, name)
    return this.addRaw(tags)
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
  and<E1>(tag: TypedTagIntersection<E1>): TypedTagIntersection<Extract<E1, E>>

  // andNamed<E1>(tag: TypedTagIntersection<E1>, name: string): TypedTagIntersection<Extract<E1, E>>

  raw(): TagIntersection

  readonly _dataType?: E

  readonly type: 'typed-intersection'
}

export interface Tag<E> extends TypedTagIntersection<E> {
  // raw tag
  readonly rawTag: string

  subSpace(name: string): TypedTagIntersection<E>

  local(): TypedTagIntersection<E>
}

const extractTagStrings = (tags: ReadonlyArray<Tag<unknown>>) => tags.map(x => x.rawTag)

export const Tag = {
  create: <E>(rawTag: string): Tag<E> => ({
    rawTag,

    subSpace: (name: string) => req(false, namedSubSpace(rawTag, name)),

    local: () => req(true, [rawTag]),

    ...req(false, [rawTag]),
  }),
}

export type TypedTagQuery<E> = TypedTagUnion<E> | TypedTagIntersection<E>

const req = <E>(onlyLocalEvents: boolean, rawTags: string[]): TypedTagIntersection<E> => {
  return {
    and: <E1>(tag: TypedTagIntersection<E1>) => {
      const other = tag.raw()

      const local = onlyLocalEvents || !!other.onlyLocalEvents
      const tags = rawTags.concat(other.tags)

      // const cast = [...tags, tag] as Tag<Extract<E1, E>>[]
      return req<Extract<E1, E>>(local, tags)
    },

    type: 'typed-intersection',

    raw: () => ({
      type: 'intersection',

      tags: rawTags,

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
  // requireTag: <E>(x: Tag<E>) => req(false, [x]),
  // requireNamed: <E>(tag: Tag<E>, name: string) => req(false, Tag.namedSubSpace(tag, name)),

  // requireLocalTag: <E>(x: Tag<E>) => req(true, [x]),
  // requireLocalNamed: <E>(tag: Tag<E>, name: string) => req(true, Tag.namedSubSpace(tag, name)),

  matchAnyOf,
}
