import axios from 'axios'
import http from 'http'
import { MyGlobal } from '../jest/setup'

type Headers = Record<string, string>
const mkHttpClient = (headers?: Headers) => {
  const apiEvent = (<MyGlobal>global).axNodeSetup.nodes[0]._private.apiEvent
  const baseURL = `${apiEvent}v2/events`
  return axios.create({ baseURL, headers })
}
export const Authorization = 'Bearer TEST_TOKEN' //FIXME replace it with the a real token

const InvalidAuthorization = 'Bearer invalid'

export const httpClient = mkHttpClient({ Authorization })

export const httpClientNoHeaders = mkHttpClient()

export const httpClientInvalidAccept = mkHttpClient({
  Accept: 'invalid',
  Authorization,
})

export const httpClientInvalidToken = mkHttpClient({ Authorization: InvalidAuthorization })

export const REQUEST_OPTIONS_QUERY: http.RequestOptions = {
  method: 'POST',
  hostname: 'localhost',
  port: 4454,
  path: '/api/v2/events/query',
  headers: {
    Accept: 'application/x-ndjson',
    'Content-Type': 'application/json',
    Authorization: 'Bearer something',
  },
}

export const REQUEST_OPTIONS_SUBSCRIBE_MONOTONIC: http.RequestOptions = {
  method: 'POST',
  hostname: 'localhost',
  port: 4454,
  path: '/api/v2/events/subscribe_monotonic',
  headers: {
    Accept: 'application/x-ndjson',
    'Content-Type': 'application/json',
    Authorization: 'Bearer something',
  },
}

export const REQUEST_OPTIONS_SUBSCRIBE: http.RequestOptions = {
  method: 'POST',
  hostname: 'localhost',
  port: 4454,
  path: '/api/v2/events/subscribe',
  headers: {
    Accept: 'application/x-ndjson',
    'Content-Type': 'application/json',
    Authorization: 'Bearer something',
  },
}
