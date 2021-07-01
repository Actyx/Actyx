/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2021 Actyx AG
 */
import { Observable } from 'rxjs'
import { Where } from './tags'
import { Metadata, PendingEmission } from './various'

/** Anything that has tags. @alpha */
export type HasTags = {
  tags: ReadonlyArray<string>
}

/**
 * Turn a `Where` into a function that can decide whether a locally emitted event matches the clause.
 *
 * We can ignore the 'local' flag since it will never exclude our local events,
 * and this method is used solely to decide whether locally emitted events are relevant.
 *
 * TODO: This will be removed once we support other means of 'I got events up to X' feedback
 *
 * @alpha
 */
export const toEventPredicate = (where: Where<unknown>) => {
  const tagSets = where.toV1WireFormat()

  return (event: HasTags) =>
    tagSets.some(tagIntersection => tagIntersection.tags.every(tag => event.tags.includes(tag)))
}

/** Create a PendingEmission object from an Observable. @internal */
export const pendingEmission = (o: Observable<Metadata[]>): PendingEmission => ({
  subscribe: o.subscribe.bind(o),
  toPromise: () => o.toPromise(),
})

/**
 * Refinement that checks whether typeof x === 'string'
 * @public
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const isString = (x: any): x is string => typeof x === 'string'

/**
 * Refinement that checks whether typeof x === 'number'
 * @public
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const isNumber = (x: any): x is number => typeof x === 'number'

/**
 * Refinement that checks whether typeof x === 'number'
 * @public
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const isBoolean = (x: any): x is boolean => typeof x === 'boolean'
