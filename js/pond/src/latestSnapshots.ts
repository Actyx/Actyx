/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import * as assert from 'assert'
import { none, Option } from 'fp-ts/lib/Option'
import { contramap, Ord } from 'fp-ts/lib/Ord'
import { EnvelopeFromStore } from './store/util'
import { EventKey, LocalSnapshot } from './types'

const assertNewer = <T>(newVal: Option<T>, oldVal: Option<T>, ord: Ord<T>) => {
  if (newVal.isSome() && oldVal.isSome()) {
    const isOlder = ord.compare(newVal.value, oldVal.value) < 0
    assert(!isOlder)
  }
}

const localSnapshotOrder = contramap((l: LocalSnapshot<{}>) => l.eventKey, EventKey.ord)

/* Holder for a pair of latest snapshots. Depending on the fish config of a given fishEventStore,
 * one or the other snapshot may not be used.
 *
 * The only invariant maintained by this class is that if a snapshot is set, it may not be replaced
 * with an older one. It may however be erased (set to none) and subsequently could be set to
 * something older.
 *
 * Most of the time, the FES will only have one or the other snapshot set. However, we don’t enforce
 * this, as there are always transition periods.
 */
export class LatestSnapshots<S, E> {
  private _semantic: Option<EnvelopeFromStore<E>> = none
  private _local: Option<LocalSnapshot<S>> = none

  get local(): Option<LocalSnapshot<S>> {
    return this._local
  }

  set local(value: Option<LocalSnapshot<S>>) {
    assertNewer(value, this._local, localSnapshotOrder)

    this._local = value
  }

  get semantic(): Option<EnvelopeFromStore<E>> {
    return this._semantic
  }

  set semantic(value: Option<EnvelopeFromStore<E>>) {
    assertNewer(value, this._semantic, EnvelopeFromStore.ord)

    this._semantic = value
  }

  clear(): void {
    this._local = none
    this._semantic = none
  }

  // Semantic takes precedence, but most of the time there should only either one of the two be set.
  fromSemanticFromLocalOrDefault<T>(
    semExtract: (s: EnvelopeFromStore<E>) => T,
    localExtract: (l: LocalSnapshot<S>) => T,
    defaultVal: T,
  ): T {
    return this.semantic
      .map(semExtract)
      .alt(this.local.map(localExtract))
      .getOrElse(defaultVal)
  }
}
