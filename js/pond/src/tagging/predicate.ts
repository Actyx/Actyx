import { Event } from '../eventstore/types'
import { Where } from './typed'

/**
 * Turn a `Where` into a function that can decide whether a locally emitted event matches the clause.
 *
 * We can ignore the 'local' flag since it will never exclude our local events,
 * and this method is used solely to decide whether locally emitted events are relevant.
 */
export const toEventPredicate = (where: Where<unknown>) => {
  const tagSets = where.toWireFormat()

  return (event: Event) =>
    tagSets.some(tagIntersection => tagIntersection.tags.every(tag => event.tags.includes(tag)))
}
