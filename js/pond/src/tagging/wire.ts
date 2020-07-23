import { SubscriptionSet } from '../subscription'
import { Where } from './typed'

export const toSubscriptionSet = (where: Where<unknown>): SubscriptionSet => {
  const wire = where.toWireFormat()

  return {
    type: 'tags',
    subscriptions: Array.isArray(wire) ? wire : [wire],
  }
}
