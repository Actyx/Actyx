import { ordNumber } from 'fp-ts/lib/Ord'
import { Observable } from 'rxjs'
import { removeOverlaps } from './removeOverlaps'

describe('removeOverlaps', () => {
  const op = (chunks: ReadonlyArray<number[]>): Promise<ReadonlyArray<ReadonlyArray<number>>> =>
    Observable.from(chunks)
      .pipe(removeOverlaps(ordNumber))
      .toArray()
      .toPromise()
  it('should work for non-overlapping chunks', async () => {
    expect(await op([[4, 5, 6], [1, 2, 3]])).toEqual([[6], [5, 4, 3], [2, 1]])
  })
  it('should work for interleaving chunks', async () => {
    expect(await op([[2, 4, 6], [1, 3, 5]])).toEqual([[6], [5], [4, 3, 2, 1]])
  })
  it('should properly deal with empty chunks', async () => {
    expect(await op([[2, 4, 6], [], [1, 3, 5]])).toEqual([[6], [], [5], [4, 3, 2, 1]])
  })
  it('should properly deal with a single chunk', async () => {
    expect(await op([[1, 2, 3, 4, 5, 6]])).toEqual([[6], [5, 4, 3, 2, 1]])
  })
})
