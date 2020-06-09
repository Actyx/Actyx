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
const shouldRunIntegrationTests = () => process.env['RUN_INTEGRATION_TESTS'] === '1'

export const skipUnlessIntegrationTesting = () => {
  if (!shouldRunIntegrationTests()) {
    test.only('skipping integration tests', () => {
      console.warn('[SKIP] skipping integration tests')
    })
  }
}

const ts = new Date(Date.now()).toISOString()
export const testSemantics = (semantics: string) => `test_${ts}_semantics__${semantics}`
export const testName = (name: string) => `test_${ts}_name__${name}`

export const mkRandId = () => {
  let result = ''
  const characters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'
  const charactersLength = characters.length
  for (let i = 0; i < 6; i++) {
    result += characters.charAt(Math.floor(Math.random() * charactersLength))
  }
  return result
}
