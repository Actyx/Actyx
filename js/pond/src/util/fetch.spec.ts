import fetchMock = require('fetch-mock')
import { fetchObs, RequestOptions } from './fetch'

describe('fetchObs', () => {
  const sampleResponseData = { sample: 'response data' }

  afterEach(() => fetchMock.restore())

  describe('GET', () => {
    it('should have correct path when using get', () => {
      const samplePath = '/sample/path'
      fetchMock.mock(samplePath, 200)

      return fetchObs(samplePath, {})
        .toPromise()
        .then(() => {
          expect(fetchMock.lastUrl()).toEqual(samplePath)
          fetchMock.restore()
        })
    })

    it('should have correct path with query params when using get', () => {
      const samplePath = '/sample/path'
      const expected = `${samplePath}?test=test%20value&ary=1&ary=2&ary=3&ary=x%20x`
      const params = {
        test: 'test value',
        ary: [1, 2, 3, 'x x'],
      }
      fetchMock.mock(expected, 200)
      return fetchObs(samplePath, {}, params)
        .toPromise()
        .then(() => {
          expect(fetchMock.lastUrl()).toEqual(expected)
        })
    })

    it('should have correct fetch options when using get', () => {
      fetchMock.mock('', 200)
      return fetchObs('', {
        method: 'GET',
        mode: 'cors',
      })
        .toPromise()
        .then(() => {
          expect(fetchMock.lastOptions()).toEqual({
            method: 'GET',
            mode: 'cors',
          })
        })
    })
  })

  describe('POST', () => {
    it('should have correct path when using post', () => {
      const samplePath = '/sample/path'
      fetchMock.mock(samplePath, 200)
      return fetchObs(samplePath, { method: 'POST' })
        .toPromise()
        .then(() => expect(fetchMock.lastUrl()).toEqual(samplePath))
    })

    it('should have correct fetch options when using post', () => {
      const data = {
        id: '123',
        ary: [4, 5, 6],
      }
      const options: RequestOptions = {
        method: 'POST',
        mode: 'cors',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(data),
      }
      fetchMock.mock('', 200)
      return fetchObs('', options, data)
        .toPromise()
        .then(() => {
          expect(fetchMock.lastOptions()).toEqual({
            method: 'POST',
            mode: 'cors',
            headers: {
              'Content-Type': 'application/json',
            },
            body: JSON.stringify(data),
          })
        })
    })

    it('should have post success response', () => {
      fetchMock.mock('', Promise.resolve(sampleResponseData))
      const result = fetchObs('', {
        method: 'POST',
      })
      return expect(result.concatMap(r => r.json()).toPromise()).resolves.toEqual(
        sampleResponseData,
      )
    })

    it('should have post fail response', () => {
      fetchMock.mock('', 500)
      const result = fetchObs('', { method: 'POST', body: '{}' })
      return expect(result.toPromise()).rejects.toMatchObject({
        status: 'error',
      })
    })
  })

  describe('HTTP client network issue', () => {
    it('should have a networkError response', () => {
      fetchMock.mock('', Promise.reject('Failed to fetch.'))
      const result = fetchObs('', {})
      return expect(result.toPromise()).rejects.toMatchObject({
        status: 'networkError',
      })
    })
  })
})
