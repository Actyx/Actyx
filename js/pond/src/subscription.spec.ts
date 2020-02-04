/* eslint-disable @typescript-eslint/no-explicit-any */
import { Subscription, SubscriptionSet, subscriptionsToEnvelopePredicate } from './subscription'
import { timerFishType } from './timerFish.support.test'
import { Envelope, FishName, Lamport, Semantics, Source, SourceId, Timestamp } from './types'

const sourceA: Source = {
  semantics: Semantics.of('a'),
  name: FishName.of(''),
  sourceId: SourceId.of('id'),
}
const sourceB: Source = {
  semantics: Semantics.of('b'),
  name: FishName.of(''),
  sourceId: SourceId.of('id'),
}
const evA: Envelope<any> = {
  source: sourceA,
  timestamp: Timestamp.of(1234),
  lamport: Lamport.of(1234),
  payload: {},
}
const evB: Envelope<any> = {
  source: sourceB,
  timestamp: Timestamp.of(1234),
  lamport: Lamport.of(1234),
  payload: {},
}

describe('subscriptionsToEnvelopePredicate', () => {
  it('must allow wildcards in all but semantics', () => {
    const sub = Subscription.of(sourceA.semantics)
    const pred = subscriptionsToEnvelopePredicate(SubscriptionSet.or([sub]))
    expect(pred(evA)).toBeTruthy()
    expect(pred(evB)).toBeFalsy()
  })
  it('must allow wildcards in name but fixed source id', () => {
    const sid = SourceId.of('id')
    const sub = Subscription.of(timerFishType, undefined, sid)
    expect(subscriptionsToEnvelopePredicate(SubscriptionSet.or([sub]))(evA)).toBeFalsy()
    const sub2 = Subscription.of(sourceA.semantics, undefined, sid)
    expect(subscriptionsToEnvelopePredicate(SubscriptionSet.or([sub2]))(evA)).toBeTruthy()
    expect(subscriptionsToEnvelopePredicate(SubscriptionSet.or([sub2]))(evB)).toBeFalsy()
  })
  it('must work for empty', () => {
    const pred = subscriptionsToEnvelopePredicate(SubscriptionSet.empty)
    expect(pred(evA)).toBeFalsy()
  })
  it('must work for all', () => {
    const pred = subscriptionsToEnvelopePredicate(SubscriptionSet.all)
    expect(pred(evA)).toBeTruthy()
  })
  it('must work for or with no subscription', () => {
    const pred = subscriptionsToEnvelopePredicate(SubscriptionSet.or([]))
    expect(pred(evA)).toBeFalsy()
  })
  it('must work for or with a single subscription', () => {
    const pred = subscriptionsToEnvelopePredicate(SubscriptionSet.or([sourceA]))
    expect(pred(evA)).toBeTruthy()
  })
  it('must work for or with multiple subscriptions', () => {
    const pred = subscriptionsToEnvelopePredicate(SubscriptionSet.or([sourceA, sourceB]))
    expect(pred(evA)).toBeTruthy()
    expect(pred(evB)).toBeTruthy()
  })
})

describe('SubscriptionSet.parse', () => {
  it('should parse subscription sets', () => {
    expect(SubscriptionSet.parse('all')).toEqual(SubscriptionSet.all)
    expect(SubscriptionSet.parse('empty')).toEqual(SubscriptionSet.empty)
    expect(SubscriptionSet.parse('a/b/c|d/e/f')).toMatchSnapshot()
    expect(SubscriptionSet.parse('a|b')).toMatchSnapshot()
  })
})
