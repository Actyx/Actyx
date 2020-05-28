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
/* eslint-disable @typescript-eslint/no-namespace */
import { Either, isLeft, right, isRight, left } from 'fp-ts/lib/Either'

expect.extend({
  isLeft(received: Either<unknown, unknown>) {
    if (isLeft(received)) {
      return {
        pass: true,
        message: () => `expected ${JSON.stringify(received, null, 2)} to be left`,
      }
    }
    return {
      pass: false,
      message: () => `expected ${JSON.stringify(received, null, 2)} to be left`,
    }
  },
  isRight(received: Either<unknown, unknown>) {
    if (isRight(received)) {
      return {
        pass: true,
        message: () => `expected ${JSON.stringify(received, null, 2)} to be right`,
      }
    }
    return {
      pass: false,
      message: () => `expected ${JSON.stringify(received, null, 2)} to be right`,
    }
  },
})

/**
 * @internal
 */
declare global {
  namespace jest {
    interface Matchers<R> {
      isLeft(): object
      isRight(): object
    }
    interface Expect {
      isLeft(): object
      isRight(): object
    }
  }
}

test('Jest extensions work', () => {
  expect(left('something')).isLeft()
  expect(right('something')).isRight()
})
