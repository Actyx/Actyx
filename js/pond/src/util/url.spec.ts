import { getUrlHashValue, getUrlQueryParams } from './url'

describe('getUrlQueryParams()', () => {
  it('should return object with query params', () => {
    expect(getUrlQueryParams('http://localhost:3333/#alarms?memStore=true&sync=true')).toEqual({
      memStore: 'true',
      sync: 'true',
    })
  })

  it('should return object with prop`s value equal to true, when no query param value', () => {
    expect(getUrlQueryParams('http://localhost:3333/#alarms?memStore=')).toEqual({ memStore: true })
    expect(getUrlQueryParams('http://localhost:3333/#alarms?memStore')).toEqual({ memStore: true })
  })

  it('should return empty object when no query params', () => {
    expect(getUrlQueryParams('http://localhost:3333/')).toEqual({})
    expect(getUrlQueryParams('http://localhost:3333/#alarms')).toEqual({})
    expect(getUrlQueryParams('http://localhost:3333/#alarms?')).toEqual({})
  })
})

describe('getUrlHashValue()', () => {
  it('should return hash string without query params', () => {
    expect(getUrlHashValue('http://localhost:3333/#alarms?xxx=yyy')).toEqual('alarms')
  })

  it('should return hash string', () => {
    expect(getUrlHashValue('http://localhost:3333/#alarms')).toEqual('alarms')
  })

  it('should return empty string when no hash', () => {
    expect(getUrlHashValue('http://localhost:3333/alarms')).toEqual('')
    expect(getUrlHashValue('http://localhost:3333/alarms/#')).toEqual('')
  })
})
