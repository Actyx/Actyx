/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */

import * as t from 'io-ts'
import {
  MultiplexedWebsocket as WS,
  RequestId,
  RequestMessage,
  ResponseMessageType,
} from '../internal_common/multiplexedWebSocket'
import { validateOrThrow } from '../util'
import { WebSocketSubjectConfig } from '../../node_modules/rxjs/webSocket'
import { map, Observable, catchError, throwError } from '../../node_modules/rxjs'

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
    return this.ws.request(serviceId, payload).pipe(
      map((msg) => msg.payload),
      catchError((err: ErrorMessage) =>
        throwError(() => new Error(JSON.stringify(err.kind || err))),
      ),
    )
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
