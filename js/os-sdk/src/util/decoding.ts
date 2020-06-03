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
import { isLeft } from 'fp-ts/lib/Either'
import { Event as EventT, OffsetMap } from '../types'
import * as t from 'io-ts'
import { PathReporter } from 'io-ts/lib/PathReporter'

/** @internal */
export const NonEmptyStringType = new t.Type<string, string, unknown>(
  'NonEmptyString',
  (input: unknown): input is string => typeof input === 'string' && input.trim().length > 0,
  (input, context) =>
    typeof input === 'string' && input.trim().length > 0
      ? t.success(input)
      : t.failure(input, context),
  t.identity,
)

/** @internal */
interface ZeroOrGreaterBrand {
  readonly ZeroOrGreater: unique symbol
}

/** @internal */
export const ZeroOrGreaterType = t.brand(
  t.number,
  (n): n is t.Branded<number, ZeroOrGreaterBrand> => n >= 0,
  'ZeroOrGreater',
)

/** @internal */
type ZeroOrGreaterType = t.TypeOf<typeof ZeroOrGreaterType>

/** @internal */
const StreamIdentifierType = t.interface({
  source: NonEmptyStringType,
  name: NonEmptyStringType,
  semantics: NonEmptyStringType,
})

/** @internal */
const EventType = t.interface({
  stream: StreamIdentifierType,
  timestamp: ZeroOrGreaterType,
  lamport: ZeroOrGreaterType,
  offset: ZeroOrGreaterType,
  payload: t.any,
})

/** @internal */
export const tryMakeEventFromApiObj = (obj: object): EventT | string => {
  const res = EventType.decode(obj)
  if (isLeft(res)) {
    return PathReporter.report(res).reduce((a, n) => a + ', ' + n)
  }
  return {
    ...res.right,
    stream: {
      streamSemantics: res.right.stream.semantics,
      streamName: res.right.stream.name,
      source: res.right.stream.source,
    },
  }
}

/** @internal */
const OffsetMapType = t.record(NonEmptyStringType, ZeroOrGreaterType)

/** @internal */
export const tryMakeOffsetMapFromApiObj = (obj: object): OffsetMap | string => {
  const res = OffsetMapType.decode(obj)
  if (isLeft(res)) {
    return PathReporter.report(res).reduce((a, n) => a + ', ' + n)
  }
  return res.right
}
