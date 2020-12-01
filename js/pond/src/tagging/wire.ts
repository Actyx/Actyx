/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { SubscriptionSet } from '../subscription'
import { Where } from './typed'

export const toSubscriptionSet = (where: Where<unknown>): SubscriptionSet => {
  const wire = where.toWireFormat()

  return {
    type: 'tags',
    subscriptions: Array.isArray(wire) ? wire : [wire],
  }
}
