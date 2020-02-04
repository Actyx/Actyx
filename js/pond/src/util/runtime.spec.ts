import { getMemoryUsage } from './runtime'

function getOrZero(map: { [key: string]: number }, key: string): number {
  const value = map[key]

  return value === undefined ? 0 : value
}

describe('runtime', () => {
  it('getMemoryUsage should get some reasonable values', () => {
    const memory = getMemoryUsage()
    const usedJSHeapSize = getOrZero(memory, 'usedJSHeapSize')
    const totalJSHeapSize = getOrZero(memory, 'totalJSHeapSize')
    expect(usedJSHeapSize).toBeGreaterThan(0)
    expect(totalJSHeapSize).toBeGreaterThanOrEqual(usedJSHeapSize)
    expect(getOrZero(memory, 'externalSize')).toBeGreaterThan(0)
    expect(getOrZero(memory, 'residentSetSize')).toBeGreaterThan(0)
  })
})
