/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */

import * as t from 'io-ts'
import * as globals from '../globals'
import {
  MultiplexedWebsocket as WS,
  RequestId,
  RequestMessage,
  ResponseMessageType,
} from '../internal_common/multiplexedWebSocket'
import { validateOrThrow } from '../util'
import { WebSocketSubjectConfig } from '../../node_modules/rxjs/webSocket'
import { concatMap, Observable, catchError, throwError, finalize } from '../../node_modules/rxjs'
import { isRight } from 'fp-ts/lib/Either'
import { stringifyError } from '../util/error'
import { GlobalInternalSymbol } from './utils'

const activeRequestInternals = globals.activeRequests[GlobalInternalSymbol]

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
type ErrorMessage = t.TypeOf<typeof ErrorMessage>

export const ResponseMessage = t.union([NextMessage, CompleteMessage, ErrorMessage])
export type ResponseMessage = t.TypeOf<typeof ResponseMessage>

export class MultiplexedWebsocket {
  ws: WS<ResponseMessage>
  constructor(
    config: WebSocketSubjectConfig<RequestMessage | ResponseMessage>,
    redialAfter: number = 2000,
  ) {
    this.ws = new WS(config, redialAfter)
  }
  request(serviceId: string, payload?: unknown): Observable<unknown> {
    const sym = Symbol()

    const request = this.ws.request(serviceId, payload).pipe(
      finalize(() => activeRequestInternals.unregister(sym)),
      concatMap((msg) => msg.payload),
      catchError((err: unknown) => {
        const e = ErrorMessage.decode(err)
        return isRight(e)
          ? throwError(() => new Error(stringifyError(e.right.kind)))
          : throwError(() => err)
      }),
    )

    activeRequestInternals.register(sym, {
      serviceId,
      payload,
      time: new Date(),
    })

    return request
  }
  close() {
    this.ws.close()
  }
  errors(): Observable<unknown> {
    return this.ws.errors
  }
}

const serializer = (msg: RequestMessage | ResponseMessage): string => {
  // rxjs websocket only has one type, but we know that we serialize only requests
  return JSON.stringify(RequestMessage.encode(<RequestMessage>msg))
}
const deserializer = (msg: MessageEvent): ResponseMessage =>
  validateOrThrow(ResponseMessage)(JSON.parse(msg.data))

export const mkConfig = (
  url: string,
): WebSocketSubjectConfig<RequestMessage | ResponseMessage> => ({
  url,
  serializer,
  deserializer,
})
