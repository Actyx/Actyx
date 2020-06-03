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
import { log } from './log'
import { ApiClientOpts, LogSeverity, SimpleLogger, SimpleLoggerOpts } from '../../types'

/** @internal */
export const createSimpleLogger = (clientOpts: ApiClientOpts) => (
  opts: SimpleLoggerOpts,
): SimpleLogger => {
  const doLog = (severity: LogSeverity) => (message: string, additionalData?: unknown) =>
    log(clientOpts)({
      entry: {
        producer: {
          name: opts.producerName,
          version: opts.producerVersion,
        },
        logName: opts.logName,
        severity,
        message,
        additionalData,
      },
      onError: err => {
        if (opts && opts.onError) {
          opts.onError(err)
        }
      },
    })

  return {
    debug: doLog(LogSeverity.DEBUG),
    info: doLog(LogSeverity.INFO),
    warn: doLog(LogSeverity.WARN),
    error: doLog(LogSeverity.ERROR),
  }
}
