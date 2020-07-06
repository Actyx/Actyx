import { TagQuery } from './pond-v2-types'

export type Tag<E> = Readonly<{
  // raw tag
  tag: string

  // underlying data type guaranteed by user
  _dataType?: E
}>

export class EmissionTags<E> {
  private tags: ReadonlyArray<string> = []

  constructor() { }

  add<E1>(tag: Tag<E>): EmissionTags<Extract<E, E1>> {
    const r = new EmissionTags<unknown>()
    r.tags = [...this.tags, tag.tag]
    return r as EmissionTags<Extract<E, E1>>
  }

  addNamed<E1>(tag: Tag<E>, name: string): EmissionTags<Extract<E, E1>> {
    const subtag = tag + ':' + name

    const r = new EmissionTags<unknown>()
    r.tags = [...this.tags, tag.tag, subtag]
    return r as EmissionTags<Extract<E, E1>>
  }

  rawTags(): ReadonlyArray<string> {
    return this.tags
  }
}

const toUnion = (q: TagQuery) => {
  if (q.type === 'union') {
    return q
  } else {
    return {
      type: 'union',
      tags: [q]
    }
  }
}

export class TypedTagQuery<E> {

  private rawQuery?: TagQuery

  union<E1>(other: TypedTagQuery<E>): TypedTagQuery<E | E1> {
    const l = this.rawQuery.type === 'union' ? this.rawQuery : { type: 'union', tags: [this.rawQuery] }
    l.tags.push(
  }

}
