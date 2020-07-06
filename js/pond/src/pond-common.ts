/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Observable } from 'rxjs'
import { ConnectivityStatus } from './eventstore/types'
import { PondState } from './pond-state'
import { Config as WaitForSwarmConfig, SplashState } from './splashState'
import { SourceId } from './types'

export type PondInfo = {
  sourceId: SourceId
}

export type PondCommon = {
  /**
   * Dispose subscription to IpfsStore
   * Store subscription needs to be unsubscribed for HMR
   */
  dispose(): Promise<void>

  /**
   * Information about the current pond
   */
  info(): PondInfo

  /**
   * Obtain an observable state of the pond.
   */
  getPondState(): Observable<PondState>

  /**
   * Obtain an observable describing connectivity status of this node.
   */
  getNodeConnectivity(...specialSources: ReadonlyArray<SourceId>): Observable<ConnectivityStatus>

  /**
   * Obtain an observable that completes when we are mostly in sync with the swarm.
   * It is recommended to wait for this on application startup, before interacting with any fish,
   * i.e. `await pond.waitForSwarmSync().toPromise()`. The intermediate states emitted
   * by the Observable can be used to display render a progress bar, for example.
   */
  waitForSwarmSync(config?: WaitForSwarmConfig): Observable<SplashState>
}
