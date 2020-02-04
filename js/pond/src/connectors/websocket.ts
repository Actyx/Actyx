/* eslint-disable @typescript-eslint/no-explicit-any */

import { BehaviorSubject, Observable, Observer, Subject } from 'rxjs'
import { WebSocketSubject, WebSocketSubjectConfig } from 'rxjs/observable/dom/WebSocketSubject'
import reconnectStrategies, { ReconnectStrategy } from '../util/reconnectStrategies'

export type ConnectionStatus = 'open' | 'closing' | 'closed'

export type WebSocketObservables<T> = {
  outgoing: Observer<any>
  incoming: Observable<T>
  statusObservable: Observable<ConnectionStatus>
}

/**
 * Creates a RxJS WebSocketSubject `subject` with an additional Observable `statusObservable`, which indicates
 * the current ConnectionStatus ('open' | 'closing' | 'closed').
 * @param {*} cfg The Config holds the url to connect to;
 * a result selector function, which maps the received MessageEvent from the WebSocket to T. If you leave it empty,
 * the RxJS standard one (JSON.parse(e.data)) will be used; and a reconnect strategy, where a custom reconnection
 * strategy for the WebSocketSubject can be specified (Note: Other errors will be passed through to the subscriber).
 */
function mkWebSocket<T>(cfg: {
  url: string
  resultSelector?: (e: MessageEvent) => T
  reconnectStrategy?: ReconnectStrategy
}): WebSocketObservables<T> {
  const { url, resultSelector, reconnectStrategy } = cfg
  const rcStrategy: ReconnectStrategy =
    reconnectStrategy || reconnectStrategies.exponentialBackoff()
  const connectionObserver: Subject<ConnectionStatus> = new BehaviorSubject<ConnectionStatus>(
    'closed',
  )
  // Using only next of the {open,closing,close}Observers is fine, as they all are of type NextObserver.
  const config: WebSocketSubjectConfig = {
    url,
    openObserver: {
      next: () => connectionObserver.next('open'),
    },
    closingObserver: {
      next: () => connectionObserver.next('closing'),
    },
    closeObserver: {
      next: () => connectionObserver.next('closed'),
    },
  }
  if (resultSelector !== undefined) {
    config.resultSelector = resultSelector as (e: MessageEvent) => any
  }
  const statusObservable: Observable<
    ConnectionStatus
  > = connectionObserver.share().distinctUntilChanged()

  const wss: WebSocketSubject<T> = new WebSocketSubject<T>(config)
  const outgoing: Observer<any> = {
    next: (value: any) => {
      wss.next(value)
    },
    error: (err: any) => {
      // the WebSocketSubject does not close the connection unless there is a code field.
      const err1 = 'code' in err ? err : { ...err, code: 1001, reason: 'application error' }
      wss.error(err1)
    },
    complete: () => {
      wss.complete()
    },
  }
  const incoming: Observable<T> = wss.retryWhen(errors => rcStrategy(errors)).share()
  return { statusObservable, incoming, outgoing }
}

export default mkWebSocket
