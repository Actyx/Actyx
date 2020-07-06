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
}
