import { AxEventService, PublishResponse, PublishResponseKey } from '../../http-client'

export const integrationTag = 'integration9'

export const randomString = (): string =>
  Math.random()
    .toString(36)
    .replace(/[^a-z]+/g, '')
    .substr(0, 5)

export type TestPayload = {
  value: string
}
const mkPayload = (value: string): TestPayload => ({ value })

export const publishWithTag = (es: AxEventService, tags: ReadonlyArray<string>) => (
  value: string,
): Promise<PublishResponse> => es.publish({ data: [{ tags, payload: mkPayload(value) }] })

export const publish = (es: AxEventService): ((value: string) => Promise<PublishResponse>) =>
  publishWithTag(es, [integrationTag, 'test:1'])

export const publishRandom = (
  es: AxEventService,
): Promise<PublishResponseKey & { payload: TestPayload }> => {
  const str = randomString()
  return publishWithTag(es, [integrationTag, 'test:1'])(str).then((response) => ({
    ...response.data[0],
    payload: mkPayload(str),
  }))
}

export const throwOnCb = (msg: string) => (...rest: unknown[]): void => {
  throw new Error(`Unexpected callback invocation. ${msg}\n ${JSON.stringify(rest)}`)
}
