import { TagSubscription } from '../subscription'
import { TypedTagQuery } from './typed'
import { TagQuery } from './untyped'

const isTyped = (i: TypedTagQuery<unknown> | TagQuery): i is TypedTagQuery<unknown> => {
  return i.type === 'typed-union' || i.type === 'typed-intersection'
}

export const toWireFormat = (
  sub: TagQuery | TypedTagQuery<unknown>,
): ReadonlyArray<TagSubscription> => {
  if (isTyped(sub)) {
    return toWireFormat(sub.raw())
  }

  switch (sub.type) {
    case 'intersection':
      return [
        {
          tags: sub.tags,
          local: !!sub.onlyLocalEvents,
        },
      ]

    case 'union':
      return sub.tags.map(
        s =>
          typeof s === 'string'
            ? { tags: [s], local: false }
            : { tags: s.tags, local: !!s.onlyLocalEvents },
      )
  }
}
