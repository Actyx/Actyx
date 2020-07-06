export type TagIntersection = Readonly<{
  type: 'intersection'
  tags: ReadonlyArray<string>
  onlyLocalEvents?: boolean
}>

export type TagUnion = Readonly<{
  type: 'union'
  tags: ReadonlyArray<string | TagIntersection>
}>

const mkUnion = (...tags: (string | TagIntersection)[]): TagUnion => ({
  type: 'union',
  tags,
})

const mkIntersection = (onlyLocalEvents: boolean) => (...tags: string[]): TagIntersection => ({
  type: 'intersection',
  tags,
  onlyLocalEvents,
})

export type TagQuery = TagUnion | TagIntersection

export const TagQuery = {
  // "What do I match?" terminology
  requireAll: mkIntersection(false),
  requireAllLocal: mkIntersection(true),
  matchAnyOf: mkUnion,

  // For internal use -- should maybe move somewhere else.
  toWireFormat: (sub: TagQuery) => {
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
  },
}
