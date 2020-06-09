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
import { skipUnlessIntegrationTesting } from '../../util/test-util'
import { Client } from '../../client'
import { LogSeverity } from '../../types'

describe('integration test: logging works', () => {
  skipUnlessIntegrationTesting()

  test(`send log messages using the SimpleLogger`, done => {
    const client = Client()
    const logger = client.consoleService.SimpleLogger({
      logName: 'testLog',
      producerName: 'integration-tests',
      producerVersion: '0.0.0',
      onError: err => {
        throw new Error(err)
      },
    })

    logger.debug('debug message', { foo: 'bar' })
    logger.warn('warn message', { foo: 'bar' })
    logger.info('info message', { foo: 'bar' })
    logger.error('error message', { foo: 'bar' })

    setTimeout(done, 2000)
  })

  test(`send log messages using the log function`, done => {
    const client = Client()
    client.consoleService.log({
      entry: {
        logName: 'testLog',
        producer: {
          name: 'integration-tests',
          version: '0.0.0',
        },
        message: 'test log message',
        severity: LogSeverity.DEBUG,
      },
      onError: err => {
        throw new Error(err)
      },
      onLogged: done,
    })
  })
})
