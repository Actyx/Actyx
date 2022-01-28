import {
  webSocket,
  WebSocketSubject,
  WebSocketSubjectConfig,
} from '../../node_modules/rxjs/webSocket'
import {
  mergeMap,
  NEVER,
  Observable,
  of as single,
  Subject,
  takeWhile,
  throwError,
} from '../../node_modules/rxjs'
import * as t from 'io-ts'
import { unreachable } from '../util'
import * as WebSocket from 'isomorphic-ws'
import log from './log'

/**
 * Unique request id to be chosen by the client. 53 bit integer. Reusing existing request id will cancel the current
 * request with that id.
 */
export const RequestId = t.number
export type RequestId = t.TypeOf<typeof RequestId>

export const enum RequestMessageType {
  Request = 'request',
  Cancel = 'cancel',
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
export type RequestMessage = t.TypeOf<typeof RequestMessage>

export const enum ResponseMessageType {
  Next = 'next',
  Error = 'error',
  Complete = 'complete',
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const formatError = (err: any) => {
  if ('code' in err && 'reason' in err) {
    return `code ${err.code}: '${err.reason}'`
  }
  if ('target' in err && 'type' in err && 'error' in err) {
    return `${err.error}`
  }
  return err
}

const summariseEvent = (e: unknown): string => {
  if (Array.isArray(e)) {
    return '[' + e.map(summariseEvent).join() + ']'
  }
  if (typeof e === 'object' && e !== null) {
    return (
      '{' +
      Object.entries(e)
        .map(([k, v]) =>
          k === 'type' ? `type:${v}` : k === 'payload' ? `payload:${summariseEvent(v)}` : k,
        )
        .join() +
      '}'
    )
  }
  return `${e}`
}

export class MultiplexedWebsocket<Res extends { requestId: number; type: ResponseMessageType }> {
  private subject: WebSocketSubject<RequestMessage | Res> | null
  private requestId = 0
  private lastDial = 0
  readonly errors = new Subject<unknown>()

  constructor(
    private config: WebSocketSubjectConfig<RequestMessage | Res>,
    private redialAfter: number,
  ) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    config.WebSocketCtor || (config.WebSocketCtor = <any>WebSocket)
    this.subject = webSocket(config)
    this.keepAlive()
  }

  private keepAlive() {
    log.ws.debug('dialling', this.config.url)
    this.lastDial = Date.now()
    this.subject?.subscribe({
      next: (msg) => log.ws.debug('received message:', summariseEvent(msg)),
      error: (err) => {
        log.ws.error('connection error:', formatError(err))
        const now = Date.now()
        const delay = Math.max(this.lastDial + this.redialAfter, now) - now
        setTimeout(() => this.keepAlive(), delay)
        this.errors.next(err)
      },
      complete: () => {
        log.ws.warn('WebSocket closed')
        const now = Date.now()
        const delay = Math.max(this.lastDial + this.redialAfter, now) - now
        setTimeout(() => this.keepAlive(), delay)
      },
    })
  }

  close() {
    const s = this.subject
    this.subject = null
    s?.complete()
  }

  request(
    serviceId: string,
    payload?: unknown,
  ): Observable<Res & { type: ResponseMessageType.Next }> {
    if (this.subject === null) {
      this.subject = webSocket(this.config)
      this.keepAlive()
    }
    if (serviceId === 'wake up') {
      // the purpose was just to start a new webSocket
      return NEVER
    }

    const requestId = this.requestId++
    const reqMsg: RequestMessage = {
      type: RequestMessageType.Request,
      requestId,
      serviceId,
      payload: payload || null,
    }
    const cancelMsg: RequestMessage = {
      type: RequestMessageType.Cancel,
      requestId,
    }

    return this.subject
      .multiplex(
        () => reqMsg,
        () => cancelMsg,
        (msg) => msg.requestId === requestId,
      )
      .pipe(
        takeWhile((msg) => msg.type !== ResponseMessageType.Complete),
        mergeMap((msg) => {
          switch (msg.type) {
            case ResponseMessageType.Next:
              return single(<Res & { type: ResponseMessageType.Next }>msg)
            case ResponseMessageType.Error:
              return throwError(() => msg)
            default:
              unreachable()
          }
        }),
      )
  }
}
