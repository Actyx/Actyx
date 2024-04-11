import reporter from 'io-ts-reporters'
import { Errors } from 'io-ts'
import { left } from 'fp-ts/lib/Either'
import { Multiaddr } from 'multiaddr'
import whatwgurl from 'whatwg-url'

export const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))
export const zip = <A, B>(as: A[], bs: B[]): [A, B][] => as.map((a, i) => [a, bs[i]])

export const zipWithPromises = <A, B>(as: A[], promises: Promise<B>[]): Promise<[A, B]>[] =>
  as.map((a, i) => promises[i].then((b: B) => [a, b]))

export const ioErrToStr = (errs: Errors): string => reporter.report(left(errs)).join(', ')

export const safeErrorToStr = (err: unknown): string => {
  if (!err) {
    return 'none'
  }
  if (typeof err === 'string') {
    return err
  }

  if (typeof err === 'object') {
    if (Object.prototype.hasOwnProperty.call(err, 'shortMessage')) {
      if (Object.prototype.hasOwnProperty.call(err, 'details')) {
        return (err as any).shortMessage + '(' + (err as any).details + ')'
      } else {
        return (err as any).shortMessage
      }
    }
  }
  return JSON.stringify(err, (_, v) => (typeof v === 'function' ? '<func>' : v)) + err
}

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

/**
 * accepts valid hostname:port
 */
export const nodeAddrValid = (addr: string) => {
  // https://url.spec.whatwg.org/#authority-state
  const record = whatwgurl.basicURLParse(addr, { stateOverride: 'authority' })
  if (!record) return false
  // to check if more than hostname:port is supplied
  // e.g. added username, password, path, schema + colon, slash, @, etc
  const reserialized = record.host + (record.port ? `:${record.port}` : '')
  if (addr !== reserialized) return false
  // now we can assume that `reserialized` is hostname:port
  try {
    // use url to catch invalid hostnames such as 1.1.1.1.1
    new URL(`http://${reserialized}`)
    return true
  } catch {
    return false
  }
}
