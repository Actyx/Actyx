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
import { DefaultClientOpts, Client } from '../../..'

test('should reject when calling nonexistent host', async () => {
  const opts = DefaultClientOpts()
  opts.Endpoints.EventService.BaseUrl = 'http://nonexistent/'
  try {
    await Client(opts).eventService.offsetsPromise()
    fail('should have thrown')
  } catch (e) {
    // expected
  }
})

test('should reject when calling nonexistent port', async () => {
  const opts = DefaultClientOpts()
  opts.Endpoints.EventService.BaseUrl = 'http://localhost:12345/'
  try {
    await Client(opts).eventService.offsetsPromise()
    fail('should have thrown')
  } catch (e) {
    // expected
  }
})
