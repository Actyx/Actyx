import { alphaNumCompare } from './sorters'
import { sorted, unsorted } from './sorters.support.test'

describe('sortAlphaNum', () => {
  it('should sort en', () => {
    expect(unsorted.sort(alphaNumCompare('en'))).toEqual(sorted)
  })

  it('should sort de', () => {
    expect(unsorted.sort(alphaNumCompare('de'))).toEqual(sorted)
  })
})
