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
  catchError,
  retryWhen,
  take,
  interval,
} from '../../node_modules/rxjs'
import * as t from 'io-ts'
import { unreachable } from '../util'
import * as WebSocket from 'isomorphic-ws'
import log from './log'
import { massageError } from '../util/error'

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

const summariseEvent = (e: unknown, level: number = 0): string => {
  if (Array.isArray(e)) {
    return (
      '[' +
      (level > 2
        ? '...'
        : e.map((x, idx) => (idx === 0 ? summariseEvent(x, level + 1) : '')).join()) +
      ']'
    )
  }
  if (typeof e === 'object' && e !== null) {
    return (
      '{' +
      Object.entries(e)
        .map(([k, v]) =>
          k === 'type'
            ? `type:${v}`
            : k === 'payload'
            ? `payload:${summariseEvent(v, level + 1)}`
            : k,
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
  private disconnected = true
  private queue: [Date, string, unknown, Subject<Res & { type: ResponseMessageType.Next }>][] = []

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
    this.disconnected = false
    this.lastDial = Date.now()
    this.subject?.subscribe({
      next: (msg) => log.ws.debug('received message:', summariseEvent(msg)),
      error: (err) => {
        log.ws.error('connection error:', massageError(err))
        this.disconnected = true
        const now = Date.now()
        const delay = Math.max(this.lastDial + this.redialAfter, now) - now
        log.ws.debug(`triggering reconnect in ${delay}ms`)
        setTimeout(() => this.subject && this.keepAlive(), delay)
        this.errors.next(err)
      },
      complete: () => {
        log.ws.warn('WebSocket closed')
        this.disconnected = true
        const now = Date.now()
        const delay = Math.max(this.lastDial + this.redialAfter, now) - now
        setTimeout(() => this.subject && this.keepAlive(), delay)
      },
    })
    this.retryAll()
  }

  close() {
    const s = this.subject
    this.subject = null
    s?.complete()
    this.queue.forEach(([_d, _s, _p, subject]) =>
      throwError(() => new Error('disconnected from Actyx')).subscribe(subject),
    )
    this.queue.length = 0
  }

  private enqueue(serviceId: string, payload: unknown) {
    const s = new Subject<Res & { type: ResponseMessageType.Next }>()
    this.queue.push([new Date(), serviceId, payload, s])
    setTimeout(() => this.prune(), this.redialAfter * 1.5)
    return s
  }

  private prune() {
    if (this.queue.length < 1 || this.queue[0][0].valueOf() + this.redialAfter * 1.5 < Date.now()) {
      return
    }
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const [_date, _serviceId, _payload, subject] = this.queue.shift()!
    throwError(() => new Error('currently disconnected from Actyx')).subscribe(subject)
  }

  private retryAll() {
    this.queue.forEach(([_date, serviceId, payload, subject]) =>
      this.request(serviceId, payload).subscribe(subject),
    )
    this.queue.length = 0
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

    if (this.disconnected) {
      log.ws.debug('enqueueing request for', serviceId)
      return this.enqueue(serviceId, payload || null)
    }
    log.ws.debug('got request for service', serviceId)

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
        catchError((err: unknown) => throwError(() => massageError(err))),
        takeWhile((msg) => msg.type !== ResponseMessageType.Complete),
        mergeMap((msg) => {
          switch (msg.type) {
            case ResponseMessageType.Next:
              return single(<Res & { type: ResponseMessageType.Next }>msg)
            case ResponseMessageType.Error: {
              const myMsg = msg as Res & { kind: { type: string; value: string } }
              if (typeof myMsg.kind === 'object' && myMsg.kind !== null) {
                const { type, value } = myMsg.kind
                if (
                  type === 'serviceError' &&
                  value === 'Channel towards event store is overloaded.'
                ) {
                  log.ws.info('retrying request for service', serviceId, 'due to rate limit')
                  return interval(100).pipe(
                    take(1),
                    mergeMap(() => this.request(serviceId, payload)),
                  )
                }
              }
              return throwError(() => msg)
            }
            default:
              unreachable()
          }
        }),
      )
  }
}
