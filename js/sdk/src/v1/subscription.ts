/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import * as t from 'io-ts'

const subscriptionToString = (subscription: Subscription): string => {
  if (subscription.name === '' && subscription.sourceId !== '') {
    throw new Error()
  }
  // TODO: escaping / and ,
  const parts = [subscription.semantics, subscription.name, subscription.sourceId].filter(
    x => x !== '',
  )
  return parts.join('/')
}

export const SubscriptionIO = t.readonly(
  t.type({
    semantics: t.string,
    // Wildcard is ""
    name: t.string,
    // Wildcard is ""
    sourceId: t.string,
  }),
)
export const Subscription = {
  toString: subscriptionToString,
}
export type Subscription = t.TypeOf<typeof SubscriptionIO>

export const TagSubscription = t.readonly(
  t.type({
    tags: t.readonlyArray(t.string),
    local: t.boolean,
  }),
)

export type TagSubscription = t.TypeOf<typeof TagSubscription>

/**
 * A set of subscriptions
 */
export const SubscriptionSetIO = t.taggedUnion(
  // FIXME: why doesn't this work with t.readonly??
  'type',
  [
    t.type({ type: t.literal('empty') }),
    t.type({ type: t.literal('all') }),
    t.type({ type: t.literal('or'), subscriptions: t.readonlyArray(SubscriptionIO) }),
    t.type({ type: t.literal('tags'), subscriptions: t.readonlyArray(TagSubscription) }),
    // To be added in the future:
    // t.type({ type: t.literal('tag_expr'), subscriptions: t.string }),
  ],
)
export type SubscriptionSet = t.TypeOf<typeof SubscriptionSetIO>

export type SubscriptionSetCompanion = {
  empty: SubscriptionSet
  all: SubscriptionSet
  or: (s: ReadonlyArray<Subscription>) => SubscriptionSet
}

export const SubscriptionSet: SubscriptionSetCompanion = {
  empty: { type: 'empty' },
  all: { type: 'all' },
  or: (subscriptions: ReadonlyArray<Subscription>) => ({ type: 'or', subscriptions }),
}
