/* eslint-disable @typescript-eslint/no-explicit-any */
import * as t from 'io-ts'
import { Observable, Observer, Subscription } from 'rxjs'
import log from '../loggers'
import { unreachable } from '../util/typescript'
import { WsStoreConfig } from './types'
import { WebSocketSubject } from './webSocketSubject'

/**
 * Unique request id to be chosen by the client. 53 bit integer. Reusing existing request id will cancel the current
 * request with that id.
 */
const RequestId = t.number
export type RequestId = t.TypeOf<typeof RequestId>

export const RequestMessage = t.readonly(
  t.intersection([
    t.type({
      type: t.string, // application specific identifier or 'cancelRequest'
      requestId: RequestId, // service identifier
      serviceId: t.string,
    }),
    t.partial({
      payload: t.unknown,
    }),
  ]),
)
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
    payload: t.unknown,
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

const validationErrorsMsgs = (input: any, decoder: string, errors: t.Errors) => {
  const validationErrors = errors.map(
    error => `[${error.context.map(({ key }) => key).join('.')}] = ${JSON.stringify(error.value)}`,
  )
  return `Validation of [${JSON.stringify(input)}] to ${decoder} failed:\n${validationErrors.join(
    '.',
  )}`
}

// Just as cast in production
export const validateOrThrow = <T>(decoder: t.Decoder<any, T>) => (value: any) => {
  if (process.env.NODE_ENV !== 'production') {
    return decoder.decode(value).fold(errors => {
      throw new Error(validationErrorsMsgs(value, decoder.name, errors))
    }, x => x)
  }
  return value as T
}

export class MultiplexedWebsocket {
  private wsSubject: WebSocketSubject<ResponseMessage>
  // eslint-disable-next-line @typescript-eslint/ban-ts-ignore
  // @ts-ignore TS6133 (declared but never read)
  private keepAlive: Subscription
  private requestCounter: RequestId = 0

  constructor({ url, protocol, onStoreConnectionClosed, reconnectTimeout }: WsStoreConfig) {
    this.wsSubject = WebSocketSubject.create(
      url,
      protocol,
      onStoreConnectionClosed,
      reconnectTimeout,
    )
    /**
     * If there are no subscribers, the actual WS connection will be torn down. Keep it open.
     */
    this.keepAlive = this.wsSubject.subscribe()
  }

  private handlers = (requestId: RequestId, serviceId: string, payload?: any) => ({
    onSubscribe: () => ({
      serviceId,
      type: 'request',
      requestId,
      payload: payload || null,
    }),
    onUnsubscribe: () => ({
      type: 'cancel',
      requestId,
    }),
    messageFilter: (response: ResponseMessage) => response.requestId.valueOf() === requestId,
  })

  /**
   * Copied from `rxjs/observable/dom/WebSocketSubject.ts`, as it was not possible to
   * *NOT* send a message, if the downstream consumer unsubscribes.
   */
  private multiplex = (
    requestType: string,
    payload: unknown,
    unSubscribeHandler: (unsubscribe: () => any) => any,
  ) => {
    const wss = this.wsSubject

    return new Observable((observer: Observer<any>) => {
      // The assigning of the request id needs to be done inside the
      // observer creation to make sure that it is not reused
      const requestId = this.requestCounter++
      const { onSubscribe, onUnsubscribe, messageFilter } = this.handlers(
        requestId,
        requestType,
        payload,
      )

      try {
        // https://github.com/ReactiveX/rxjs/blob/5.x/src/observable/dom/WebSocketSubject.ts on rxjs 5, no serializer is provided.
        // Therefore we provide one by hand. Caution caution when upgrading to rxjs 6, where this defaults to JSON.stringify.
        // Also other places in this file (look for JSON.stringify's, they should be redundant on rxjs 6)
        const subMsg = onSubscribe()
        log.ws.debug('About to subscribe %j', subMsg)
        wss.next(subMsg as ResponseMessage)
      } catch (err) {
        observer.error(err)
      }

      const subscription = wss.subscribe(
        x => {
          try {
            if (messageFilter(x)) {
              log.ws.debug('Received %o', x)
              observer.next(x)
            }
          } catch (err) {
            log.ws.error('Received error during message delivery %o', err)
            observer.error(err)
          }
        },
        err => observer.error(err),
        () => observer.complete(),
      )

      return () => {
        try {
          log.ws.debug('About to unsubscribe from requestId: %s', requestId)
          const unsubMsg = unSubscribeHandler(onUnsubscribe)
          if (unsubMsg) {
            log.ws.debug('About to unsubscribe with %j', unsubMsg)
            wss.next(unsubMsg as ResponseMessage)
          }
        } catch (err) {
          log.ws.debug('Unsubscribe error %o', err)
          observer.error(err)
        }
        subscription.unsubscribe()
      }
    })
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
    return this.multiplex(
      requestType,
      payload,
      unSubscribe => (upstreamCompletedOrError ? undefined : unSubscribe()),
    )
      .map(validateOrThrow(ResponseMessage))
      .takeWhile(res => {
        const isComplete = res.type === ResponseMessageType.Complete
        if (isComplete) {
          upstreamCompletedOrError = true
        }
        return !isComplete
      })
      .map(res => {
        switch (res.type) {
          case ResponseMessageType.Next:
            return res.payload
          case ResponseMessageType.Error:
            upstreamCompletedOrError = true
            log.ws.error(JSON.stringify(res.kind))
            throw new Error(JSON.stringify(res.kind)) // TODO: add context to msg?
          default:
            return unreachable()
        }
      })
  }
}
