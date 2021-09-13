/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */

/* eslint-disable @typescript-eslint/no-explicit-any */
import * as t from 'io-ts'
import { Observable, Observer, Subscription } from '../../node_modules/rxjs'
import log from '../internal_common/log'
import { WsStoreConfig } from '../internal_common/types'
import { WebSocketWrapper } from '../internal_common/webSocketWrapper'
import { validateOrThrow } from '../util'
import { unreachable } from '../util/typescript'

/**
 * Unique request id to be chosen by the client. 53 bit integer. Reusing existing request id will cancel the current
 * request with that id.
 */
const RequestId = t.number
export type RequestId = t.TypeOf<typeof RequestId>

export const enum RequestMessageType {
  Request = 'request',
  Cancel = 'cancel',
  Authenticate = 'authenticate',
}

const DoRequestMsg = t.type({
  type: t.literal(RequestMessageType.Request),
  requestId: RequestId,
  serviceId: t.string, // Service the request is aimed at
  payload: t.unknown,
})

const CancelMsg = t.type({
  type: t.literal(RequestMessageType.Cancel),
  requestId: RequestId,
})

export const RequestMessage = t.union([DoRequestMsg, CancelMsg])

export type Request = t.TypeOf<typeof RequestMessage>

export const enum ResponseMessageType {
  Next = 'next',
  Error = 'error',
  Complete = 'complete',
}

const NextMessage = t.readonly(
  t.type({
    type: t.literal(ResponseMessageType.Next),
    requestId: RequestId,
    payload: t.array(t.unknown),
  }),
)

const CompleteMessage = t.readonly(
  t.type({
    type: t.literal(ResponseMessageType.Complete),
    requestId: RequestId,
  }),
)

const ErrorMessage = t.readonly(
  t.type({
    type: t.literal(ResponseMessageType.Error),
    requestId: RequestId,
    // TODO refine, we have the following on the Rust side:
    // UnknownEndpoint { endpoint: String },
    // InternalError,
    // BadRequest,
    // ServiceError { value: Value },
    kind: t.unknown,
  }),
)

export const ResponseMessage = t.union([NextMessage, CompleteMessage, ErrorMessage])
export type ResponseMessage = t.TypeOf<typeof ResponseMessage>

export class MultiplexedWebsocket {
  private wsSubject: WebSocketWrapper<Request, ResponseMessage>
  private responseProcessor: Subscription
  private requestCounter: RequestId = 0

  private listeners: Record<RequestId, Observer<ResponseMessage>> = {}

  private clearListeners = (action: (o: Observer<ResponseMessage>) => void): void => {
    Object.values(this.listeners).forEach(action)
    this.listeners = {}
  }

  close = () => {
    this.responseProcessor.unsubscribe()
    this.wsSubject.close()

    this.clearListeners(l => l.complete())
  }

  constructor({ url, protocol, onStoreConnectionClosed, reconnectTimeout }: WsStoreConfig) {
    log.ws.info('establishing Pond API WS', url)
    this.wsSubject = WebSocketWrapper(url, protocol, onStoreConnectionClosed, reconnectTimeout)

    /**
     * If there are no subscribers, the actual WS connection will be torn down. Keep it open.
     * MUST OVERRIDE the `error` function, because the default empty Observer will RETHROW errors,
     * which blows up the pipeline instead of bubbling the error up into user code.
     */
    this.responseProcessor = this.wsSubject.responses.subscribe({
      next: response => {
        const listener = this.listeners[response.requestId]
        if (listener) {
          listener.next(response)
        } else {
          log.ws.warn('No listener registered for message ' + JSON.stringify(response))
        }
      },
      error: err => {
        log.ws.error('Raw websocket communication error:', err)

        this.clearListeners(l => l.error(err))
      },
    })
  }

  private handlers = (
    requestId: RequestId,
    serviceId: string,
    payload?: any,
  ): [t.TypeOf<typeof DoRequestMsg>, t.TypeOf<typeof CancelMsg>] => [
    {
      serviceId,
      type: RequestMessageType.Request,
      requestId,
      payload: payload || null,
    },
    {
      type: RequestMessageType.Cancel,
      requestId,
    },
  ]

  /**
   * Copied from `rxjs/observable/dom/WebSocketSubject.ts`, as it was not possible to
   * *NOT* send a message, if the downstream consumer unsubscribes.
   */
  private multiplex = (
    requestType: string,
    payload: unknown,
    shouldCancelUpstream: () => boolean,
  ) => {
    const wss = this.wsSubject

    const listeners = this.listeners

    const requestId = this.requestCounter++

    const [doReq, cancelReq] = this.handlers(requestId, requestType, payload)

    const res = new Observable((observer: Observer<ResponseMessage>) => {
      listeners[requestId] = observer

      return () => {
        try {
          log.ws.debug('About to unsubscribe from requestId: %s', requestId)

          if (shouldCancelUpstream()) {
            log.ws.debug('About to unsubscribe with %j', cancelReq)
            wss.sendRequest(cancelReq)
          } else {
            log.ws.debug(
              'RequestId %s was cancelled by upstream, not sending a cancelMsg',
              requestId,
            )
          }
        } catch (err) {
          log.ws.debug('Unsubscribe error %o', err)
          observer.error(err)
        }
        delete listeners[requestId]
      }
    })

    try {
      const subMsg = doReq
      log.ws.debug('About to subscribe %j', subMsg)
      wss.sendRequest(subMsg)
    } catch (err) {
      return Observable.throw(err)
    }

    return res
  }

  request: (requestType: string, payload?: unknown) => Observable<unknown> = (
    requestType,
    payload,
  ) => {
    /**
     *  If the WS stream _completes_, consumers always onunsubscribe from the underlying WsSubject, thus we can't
     *  use normal rxjs onUnsubscribe semantics. If the stream was cancelled downstream,
     *  we need to send a `cancelRequest` to the other side. The `isCompleted` boolean is to distinguish
     *  these cases.
     */

    let upstreamCompletedOrError = false
    return this.multiplex(requestType, payload, () => !upstreamCompletedOrError)
      .map(validateOrThrow(ResponseMessage))
      .takeWhile(res => {
        const isComplete = res.type === ResponseMessageType.Complete
        if (isComplete) {
          upstreamCompletedOrError = true
        }
        return !isComplete
      })
      .mergeMap(res => {
        switch (res.type) {
          case ResponseMessageType.Next:
            return res.payload
          case ResponseMessageType.Error:
            upstreamCompletedOrError = true
            log.ws.error(JSON.stringify(res.kind))
            return Observable.throw(new Error(JSON.stringify(res.kind))) // TODO: add context to msg?
          default:
            return unreachable()
        }
      })
  }
}
