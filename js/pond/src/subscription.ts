/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import * as t from 'io-ts'
import { always } from 'ramda'
import { Event } from './eventstore/types'
import { Envelope, FishName, Semantics, SourceId } from './types'

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

const parseSubscription = (text: string): Subscription => {
  const parts = text.trim().split('/')
  if (parts.length < 1 || parts.length > 3) {
    throw new Error()
  }
  const [semantics, name, sourceId] = parts
  return {
    semantics: Semantics.of(semantics),
    name: FishName.of(name),
    sourceId: SourceId.of(sourceId),
  }
}

export const SubscriptionIO = t.readonly(
  t.type({
    semantics: Semantics.FromString,
    // Wildcard is ""
    name: FishName.FromString,
    // Wildcard is ""
    sourceId: SourceId.FromString,
  }),
)
export const Subscription = {
  parse: parseSubscription,
  toString: subscriptionToString,
}
export type Subscription = t.TypeOf<typeof SubscriptionIO>

export const TagSubscription = t.readonly(
  t.type({
    tags: t.readonlyArray(t.string),
    local: t.boolean,
  }),
)

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
  parse: (text: string, separator?: string) => SubscriptionSet
}

const alwaysFalse = always(false)
const alwaysTrue = always(true)

const mkEnvelopePredicate: (s: Subscription) => (e: Envelope<any>) => boolean = s => {
  if (s.name !== '' && s.sourceId !== '') {
    // match everything
    return e =>
      e.source.semantics === s.semantics &&
      e.source.name === s.name &&
      e.source.sourceId === s.sourceId
  }
  if (s.sourceId !== '' && s.name === '') {
    // only name is ''
    return e => e.source.semantics === s.semantics && e.source.sourceId === s.sourceId
  }
  if (s.name !== '') {
    // match semantics and name
    return e => e.source.semantics === s.semantics && e.source.name === s.name
  }
  if (s.sourceId === '') {
    // match just semantics
    return e => e.source.semantics === s.semantics
  }
  throw new Error('Given combination of semantics / name / sourceId is not supported!')
}

const mkEventPredicate: (s: Subscription) => (e: Event) => boolean = s => {
  const envPredicate = mkEnvelopePredicate(s)
  // Just use Event in Envelope form.
  return e =>
    envPredicate({
      ...e,
      source: e,
    })
}

const mkSourceIdPredicate: (s: Subscription) => (sId: SourceId) => boolean = s => sId =>
  s.sourceId === '' ? true : s.sourceId === sId

const parseSubscriptions = (text: string, separator: string = '|'): SubscriptionSet => {
  const text1 = text.trim()
  if (text1 === 'all') {
    return SubscriptionSet.all
  } else if (text1 === 'empty') {
    return SubscriptionSet.empty
  } else {
    return SubscriptionSet.or(text.split(separator).map(Subscription.parse))
  }
}

export const SubscriptionSet: SubscriptionSetCompanion = {
  empty: { type: 'empty' },
  all: { type: 'all' },
  or: (subscriptions: ReadonlyArray<Subscription>) => ({ type: 'or', subscriptions }),
  parse: parseSubscriptions,
}

export const subscriptionsToPredicate: <T>(
  ss: SubscriptionSet,
  mkPredicate: (s: Subscription) => (t: T) => boolean,
) => (t: T) => boolean = (subscriptionSet, mkPredicate) => {
  switch (subscriptionSet.type) {
    case 'tags':
      throw new Error('tags case should not be passed to this function')
    case 'empty':
      return alwaysFalse
    case 'all':
      return alwaysTrue
    case 'or': {
      const { subscriptions } = subscriptionSet
      if (subscriptions.length === 0) {
        return alwaysFalse
      }
      if (subscriptions.length === 1) {
        return mkPredicate(subscriptionSet.subscriptions[0])
      }
      const predicates = subscriptions.map(mkPredicate)
      return ev => {
        /**
         * This used to be predicates.some(p => p(t)), but we want to avoid a closure
         * being created on each invocation of the predicate. It seems that v8 is not
         * smart enough to inline predicates.some()...
         */

        for (let i = 0; i < predicates.length; i++) {
          if (predicates[i](ev)) {
            return true
          }
        }
        return false
      }
    }
  }
}
export const subscriptionsToEnvelopePredicate = (ss: SubscriptionSet) => {
  if (ss.type === 'tags') {
    throw new Error('no tag support for envelopes!')
  }

  return subscriptionsToPredicate(ss, mkEnvelopePredicate)
}

export const subscriptionsToEventPredicate = (ss: SubscriptionSet) => {
  if (ss.type === 'tags') {
    // we can ignore the 'local' flag since it will never exclude our local events
    // and this method is used solely to decide whether locally emitted events are relevant
    return (event: Event) =>
      ss.subscriptions.some(tagIntersection =>
        tagIntersection.tags.every(tag => event.tags.includes(tag)),
      )
  }

  return subscriptionsToPredicate(ss, mkEventPredicate)
}

export const subscriptionsToSourceIdPredicate = (ss: SubscriptionSet) => {
  if (ss.type === 'tags') {
    throw new Error('tag-based subs cannot discern source id like this!')
  }

  return subscriptionsToPredicate(ss, mkSourceIdPredicate)
}
