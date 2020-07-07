import { TagSubscription } from '../subscription'
import { TypedTagQuery } from './typed'
import { TagIntersection, TagQuery, TagUnion } from './untyped'

const unionToWire = (sub: TagUnion) =>
  sub.tags.map(
    s =>
      typeof s === 'string'
        ? { tags: [s], local: false }
        : { tags: s.tags, local: !!s.onlyLocalEvents },
  )

const intersectionToWire = (sub: TagIntersection) => [
  {
    tags: sub.tags,
    local: !!sub.onlyLocalEvents,
  },
]

export const toWireFormat = (
  sub: TagQuery | TypedTagQuery<unknown>,
): ReadonlyArray<TagSubscription> => {
  switch (sub.type) {
    case 'typed-intersection':
      return intersectionToWire(sub.raw())
    case 'intersection':
      return intersectionToWire(sub)

    case 'typed-union':
      return unionToWire(sub.raw())
    case 'union':
      return unionToWire(sub)
  }
}
