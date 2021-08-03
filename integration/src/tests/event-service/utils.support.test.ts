import { AxEventService, EventResponse, trialManifest } from '../../http-client'
import { randomString } from '../../util'

type Response<T> = Omit<EventResponse, 'payload'> & { payload: T }

export const publish = <T>(
  es: AxEventService,
  clientId: string,
): ((...payloads: T[]) => Promise<Response<T>[]>) => (...payloads) => {
  const tags = [mySuite(), `client:${clientId}`, testName(), 'test']
  const data = payloads.map((payload) => ({ tags, payload }))
  return es.publish({ data }).then((response) =>
    response.data.map<Response<T>>((metadata, i) => ({
      type: 'event',
      ...metadata,
      tags,
      appId: trialManifest.appId as string,
      payload: payloads[i],
    })),
  )
}

export const publishRandom = (
  es: AxEventService,
  clientId: string,
): Promise<Response<{ value: string }>> =>
  publish<{ value: string }>(
    es,
    clientId,
  )({ value: randomString() }).then((response) => response[0])

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
