/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */

/**
 * In place merge sort of two ordered arrays. After calling this method, out
 * will be properly ordered according to ord.
 *
 * @param l array sorted according to ord. Will not be modified!
 * @param r array sorted according to ord. Will not be modified!
 * @param out array containing a concatenation of l and r. Will be modified in place!
 * @param ord order for l, r and out
 *
 * @returns the highest index at which out did not have to be changed
 */
export function mergeSortedInto<K>(
  l: ReadonlyArray<K>, // original events
  r: ReadonlyArray<K>, // new events
  out: K[], // original events concatenated with new events, to be modified in place
  ord: (a: K, b: K) => number, // order
): number {
  // out must be concatenation of l and r
  // out.length == l.length + a.length
  let li = 0
  let ri = 0
  let ro = l.length // index of ri element in out
  let i = 0
  let w = -1
  while (i < out.length) {
    if (li < l.length) {
      if (ri < r.length) {
        const o = ord(l[li], r[ri])
        if (o < 0) {
          // we are taking from l, so it could be that everything is still ok
          if (i === li) {
            // already at the right place. No need to assign
            w = i
          } else {
            out[i] = l[li]
          }
          li++
        } else if (o > 0) {
          out[i] = r[ri]
          ro++
          ri++
        } else {
          // log.pond.error('Got the same event twice:', l[li])
          // getting a duplicate
          if (i === li) {
            // everything still fine
            w = i
          } else {
            // prefer the older event
            out[i] = l[li]
          }
          // now remove the duplicate entry from the `out` array and progress
          out.splice(ro, 1)
          li++
          ri++
        }
      } else {
        if (i === li) {
          w = i
        } else {
          out[i] = l[li]
        }
        li++
      }
    }
    // there does not need to be an else case, since when we are copying from
    // r while l is exhausted things are guaranteed to be in the right place already!
    i += 1
  }
  return w
}
