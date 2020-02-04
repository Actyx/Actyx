import { getDualOrd, Ord } from 'fp-ts/lib/Ord'
import { Observable } from 'rxjs'
import { OperatorFunction } from 'rxjs/interfaces'
import { retainInPlaceAndGetRemoved } from './array'

interface ScanState<T> {
  readonly buffer: T[]
  readonly emit: ReadonlyArray<T>
}

/**
 * Take a stream of chunks sorted by some order ord (highest last)
 * that is sorted by by decreasing max elements, and emit non-overlapping
 * chunks that are sorted in descending order
 */
export const removeOverlaps = <T>(
  ord: Ord<T>,
): OperatorFunction<ReadonlyArray<T>, ReadonlyArray<T>> => chunks => {
  const dualOrd = getDualOrd(ord)
  return chunks
    .concat(Observable.of(undefined))
    .scan<ReadonlyArray<T> | undefined, ScanState<T>>(
      (acc, value) => {
        // flush the buffer at the end
        if (value === undefined) {
          acc.buffer.sort(dualOrd.compare)
          return {
            buffer: [],
            emit: acc.buffer,
          }
        }
        // don't do anything for an empty chunk, but make sure not to emit twice!
        if (value.length === 0) {
          return {
            buffer: acc.buffer,
            emit: [],
          }
        }
        const maxEmit = value[value.length - 1]

        acc.buffer.push(...value)
        const emit = retainInPlaceAndGetRemoved(acc.buffer, x => ord.compare(x, maxEmit) < 0)

        emit.sort(dualOrd.compare)
        return { buffer: acc.buffer, emit }
      },
      { buffer: [], emit: [] },
    )
    .map(x => x.emit)
}
