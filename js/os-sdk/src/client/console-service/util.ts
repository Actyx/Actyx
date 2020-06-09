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
import { LogEntryDraft, LogSeverity } from '../../types'
import * as apiTypes from './api-types'

/** @internal */
const severityToStr = (severity: LogSeverity): string => {
  switch (severity) {
    case LogSeverity.DEBUG:
      return 'debug'
    case LogSeverity.WARN:
      return 'warn'
    case LogSeverity.INFO:
      return 'info'
    case LogSeverity.ERROR:
      return 'error'
  }
}

/** @internal */
export const logEntryDraftToApiObj = (logEntryDraft: LogEntryDraft): apiTypes.ApiLogEntryDraft => {
  const apiED: apiTypes.ApiLogEntryDraft = {
    severity: severityToStr(logEntryDraft.severity),
    logName: logEntryDraft.logName,
    message: logEntryDraft.message,
    labels: logEntryDraft.labels ? logEntryDraft.labels : {},
    producerName: logEntryDraft.producer.name,
    producerVersion: logEntryDraft.producer.version,
  }
  if (logEntryDraft.timestamp) {
    apiED.logTimestamp = logEntryDraft.timestamp.toISOString()
  }
  if (logEntryDraft.additionalData) {
    apiED.additionalData = logEntryDraft.additionalData
  }

  return apiED
}
