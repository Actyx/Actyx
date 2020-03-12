/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */

import { contramap, Ord } from 'fp-ts/lib/Ord'
import { Envelope, EventKey, Psn, Timestamp } from '../types'

/**
 * Order for envelopes
 *
 * Envelopes are considered equal when when their event key properties are equal without considering
 * the content of the payload. Having two events that have these fields equal yet a different
 * payload would be a grave bug in our system.
 */
const ordEnvelopeFromStore: Ord<EnvelopeFromStore<any>> = contramap(
  EventKey.fromEnvelope,
  EventKey.ord,
)

export type EnvelopeFromStore<E> = Envelope<E> & {
  readonly psn: Psn
}

export const EnvelopeFromStore = {
  ord: ordEnvelopeFromStore,
}

export type EventToStore = {
  readonly timestamp: Timestamp
  readonly payload: any
}

export type StoredEvent = EventToStore & {
  readonly psn: Psn
}
// https://github.com/gcanti/io-ts/issues/216#issuecomment-471497998
import * as t from 'io-ts'

// EnumType Class
export class EnumType<A> extends t.Type<A> {
  public readonly _tag: 'EnumType' = 'EnumType'
  public enumObject!: object
  public constructor(e: object, name?: string) {
    super(
      name || 'enum',
      (u): u is A => Object.values(this.enumObject).some(v => v === u),
      (u, c) => (this.is(u) ? t.success(u) : t.failure(u, c)),
      t.identity,
    )
    this.enumObject = e
  }
}

// simple helper function
export const createEnumType = <T>(e: object, name?: string) => new EnumType<T>(e, name)
