import reporter from 'io-ts-reporters'
import { Errors } from 'io-ts'
import { left } from 'fp-ts/lib/Either'
import { Multiaddr } from 'multiaddr'
export const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))
export const zip = <A, B>(as: A[], bs: B[]): [A, B][] => as.map((a, i) => [a, bs[i]])

export const zipWithPromises = <A, B>(as: A[], promises: Promise<B>[]): Promise<[A, B]>[] =>
  as.map((a, i) => promises[i].then((b: B) => [a, b]))

export const ioErrToStr = (errs: Errors): string => reporter.report(left(errs)).join(', ')

export const safeErrorToStr = (err: unknown) =>
  !err
    ? undefined
    : typeof (err as any) === 'string'
    ? err
    : !err
    ? ''
    : typeof (err as any).toString === 'function'
    ? (err as any).toString()
    : JSON.stringify(err, (_, v) => (typeof v === 'function' ? '<func>' : v))

export const isValidMultiAddr = (str: string): boolean => {
  try {
    const m = new Multiaddr(str)
    return true
  } catch (error) {
    return false
  }
}

export const isValidMultiAddrWithPeerId = (str: string): boolean => {
  try {
    const m = new Multiaddr(str)
    return m.getPeerId() !== null
  } catch (error) {
    return false
  }
}
