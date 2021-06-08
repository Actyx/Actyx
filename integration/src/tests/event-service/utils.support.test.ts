import { AxEventService, PublishResponse, PublishResponseKey } from '../../http-client'
import { randomString } from '../../util'

export type TestPayload = {
  value: string
}
const mkPayload = (value: string): TestPayload => ({ value })

export const publishWithTag = (es: AxEventService, tags: ReadonlyArray<string>) => (
  value: string,
): Promise<PublishResponse> => es.publish({ data: [{ tags, payload: mkPayload(value) }] })

export const publish = (es: AxEventService): ((value: string) => Promise<PublishResponse>) =>
  publishWithTag(es, [mySuite(), testName()])

export const publishRandom = (
  es: AxEventService,
): Promise<PublishResponseKey & { payload: TestPayload }> => {
  const str = randomString()
  return publishWithTag(es, [mySuite(), testName()])(str).then((response) => ({
    ...response.data[0],
    payload: mkPayload(str),
  }))
}

export const throwOnCb = (msg: string) => (...rest: unknown[]): void => {
  throw new Error(`Unexpected callback invocation. ${msg}\n ${JSON.stringify(rest)}`)
}

/**
 * Get the current test suite (file)name, which should generally be used to tag events from this suite.
 */
export const mySuite = (): string => {
  // eslint-disable-next-line @typescript-eslint/consistent-type-assertions, @typescript-eslint/no-explicit-any
  const state = (<any>expect).getState()
  let testName: string = state.testPath
  if (testName.startsWith(process.cwd())) {
    testName = `<cwd>` + testName.substr(process.cwd().length)
  }
  return testName
}

export const testName = (): string => {
  // eslint-disable-next-line @typescript-eslint/consistent-type-assertions, @typescript-eslint/no-explicit-any
  const state = (<any>expect).getState()
  return state.currentTestName
}

// How long we are going to wait for the remote event service endpoint to answer our requests.
// This applies for tests that assert "nothing emitted" as well as for tests that look for single items inside response streams that do not end.
export const genericCommunicationTimeout = 10_000
