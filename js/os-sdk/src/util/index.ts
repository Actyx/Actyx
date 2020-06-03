/*
 * Copyright 2020 Actyx AG
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
import {
  StreamIdentifier,
  Event,
  OffsetMap,
  Ordering,
  EventDraft,
  Subscription,
  LogOpts,
  LogEntryDraft,
} from '../types'

export { tryMakeEventFromApiObj, tryMakeOffsetMapFromApiObj } from './decoding'

/** @internal */
export const mkStreamIdentifier = (
  streamSemantics: string,
  streamName: string,
  source: string,
): StreamIdentifier => ({ source, streamName, streamSemantics })

/** @internal */
export const mkEvent = (
  stream: StreamIdentifier,
  timestamp: number,
  lamport: number,
  offset: number,
  payload: unknown,
): Event => ({
  stream,
  timestamp,
  lamport,
  offset,
  payload,
})

/** @internal */
export const mkEventDraft = (
  streamSemantics: string,
  streamName: string,
  payload: unknown,
): EventDraft => ({
  streamSemantics,
  streamName,
  payload,
})

/** @internal */
export const mkSubscriptionApiObj = (subscriptions: Subscription | Subscription[]): object[] => {
  const subs: Subscription[] = Array.isArray(subscriptions) ? subscriptions : [subscriptions]
  const subsObj: object[] = []
  subs.map(filter => {
    let sub = {}
    if (filter.source !== undefined && typeof filter.source === 'string') {
      sub = { ...sub, source: filter.source }
    }
    if (filter.streamName !== undefined && typeof filter.streamName === 'string') {
      sub = { ...sub, name: filter.streamName }
    }
    if (filter.streamSemantics !== undefined && typeof filter.streamSemantics === 'string') {
      sub = { ...sub, semantics: filter.streamSemantics }
    }
    subsObj.push(sub)
  })
  return subsObj
}

/** @internal */
export const mkOffsetMapApiObj = (offsetMap: OffsetMap): object => {
  return offsetMap
}

/** @internal */
export const orderingToApiStr = (ordering: Ordering): string => {
  return ordering
}

/** @internal */
const _getAxEventServiceUriFromEnv = (): string | null => {
  const uri = process.env['AX_EVENT_SERVICE_URI']

  if (!uri) {
    return null
  }

  if (!uri.endsWith('/')) {
    return uri + '/'
  }

  return uri
}

/** @internal */
const _getAxConsoleServiceUriFromEnv = (): string | null => {
  const uri = process.env['AX_CONSOLE_SERVICE_URI']

  if (!uri) {
    return null
  }

  if (!uri.endsWith('/')) {
    return uri + '/'
  }

  return uri
}

/** @internal */
const _getAxEventServiceUriFromInjectedAx = (): string | null => {
  try {
    // Don't put on one-line since lint:fix won't accept it
    if (
      // eslint-disable-next-line
      // @ts-ignore
      typeof window === undefined ||
      // eslint-disable-next-line
      // @ts-ignore
      !window.ax ||
      // eslint-disable-next-line
      // @ts-ignore
      !window.ax.eventServiceUri ||
      // eslint-disable-next-line
      // @ts-ignore
      typeof window.ax.eventServiceUri !== 'string'
    ) {
      return null
    }
  } catch (error) {
    return null
  }

  // eslint-disable-next-line
  // @ts-ignore
  const uri: string = window.ax.eventServiceUri

  if (!uri.endsWith('/')) {
    return uri + '/'
  }

  return uri
}

/** @internal */
const _getAxConsoleServiceUriFromInjectedAx = (): string | null => {
  try {
    // Don't put on one-line since lint:fix won't accept it
    if (
      // eslint-disable-next-line
      // @ts-ignore
      typeof window === undefined ||
      // eslint-disable-next-line
      // @ts-ignore
      !window.ax ||
      // eslint-disable-next-line
      // @ts-ignore
      !window.ax.eventServiceUri ||
      // eslint-disable-next-line
      // @ts-ignore
      typeof window.ax.eventServiceUri !== 'string'
    ) {
      return null
    }
  } catch (error) {
    return null
  }

  // eslint-disable-next-line
  // @ts-ignore
  const uri: string = window.ax.eventServiceUri

  if (!uri.endsWith('/')) {
    return uri + '/'
  }

  return uri
}

/** @internal
 * This tries the injected `ax` object first, then the environment, and then
 * returns the default defined in constants.
 */
export const getAxEventServiceUri = (defaultUri: string): string => {
  let uri = _getAxEventServiceUriFromInjectedAx()
  if (uri) {
    return uri
  }

  uri = _getAxEventServiceUriFromEnv()
  if (uri) {
    return uri
  }

  return defaultUri
}

/** @internal
 * This tries the injected `ax` object first, then the environment, and then
 * returns the default defined in constants.
 */
export const getAxConsoleServiceUri = (defaultUri: string): string => {
  let uri = _getAxConsoleServiceUriFromInjectedAx()
  if (uri) {
    return uri
  }

  uri = _getAxConsoleServiceUriFromEnv()
  if (uri) {
    return uri
  }

  return defaultUri
}

/** @internal
 */
export const isLogEntryDraft = (e: LogOpts | LogEntryDraft): e is LogEntryDraft =>
  (e as LogEntryDraft).severity !== undefined
