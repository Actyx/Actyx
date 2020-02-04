export const binarySearch: <T>(
  array: ReadonlyArray<T>,
  element: T,
  compare: (element1: T, element2: T) => number,
) => number = (a, e, c) => {
  let m = 0
  let n = a.length - 1
  while (m <= n) {
    const k = (n + m) >> 1
    const cmp = c(e, a[k])
    if (cmp > 0) {
      m = k + 1
    } else if (cmp < 0) {
      n = k - 1
    } else {
      return k
    }
  }
  return -m - 1
}

export const getInsertionIndex: <T, I>(
  array: ReadonlyArray<T>,
  element: I,
  compare: (element1: T, element2: I) => number,
) => number = (a, e, c) => {
  let low = 0
  let high = a.length

  while (low < high) {
    const mid = (low + high) >>> 1
    if (c(a[mid], e) < 0) low = mid + 1
    else high = mid
  }
  return low
}
