import { RwLock } from './rwlock'

const mkPromise = <T>(): [Promise<T>, (t: T) => void] => {
  // eslint-disable-next-line @typescript-eslint/no-empty-function
  let resolve = () => {}
  const promise = new Promise<T>((res) => (resolve = res))
  return [promise, resolve]
}

const delay = () => new Promise((res) => setTimeout(() => res(), 100))

describe('RwLock', () => {
  it('should start out unlocked', () => {
    const lock = new RwLock()
    expect(lock.state()).toBe('idle')
  })
  it('should lock', async () => {
    const lock = new RwLock()
    const locked = await lock.readLock(() => lock.state())
    expect(locked).toBe('read')
    expect(lock.state()).toBe('idle')
  })
  it('should write lock', async () => {
    const lock = new RwLock()
    const locked = await lock.writeLock(() => lock.state())
    expect(locked).toBe('write')
    expect(lock.state()).toBe('idle')
  })
  it('should mutually exclude writers', async () => {
    const lock = new RwLock()
    const [value1, set1] = mkPromise<number>()
    const prom1 = lock.writeLock(() => value1)
    expect(lock.state()).toBe('write')
    let value2 = 0
    const prom2 = lock.writeLock(() => (value2 = 1))
    expect(value2).toBe(0)
    set1(42)
    expect(await prom1).toBe(42)
    await prom2
    expect(value2).toBe(1)
    expect(lock.state()).toBe('idle')
  })
  it('should mutually exclude writer and readers', async () => {
    const lock = new RwLock()
    // lock for reading and keep locked
    const [value1, set1] = mkPromise<number>()
    const prom1 = lock.readLock(() => value1)
    expect(lock.state()).toBe('read')
    // check that readers can still run
    const [value1a, set1a] = mkPromise<number>()
    let value1v = 100
    const prom1a = lock.readLock(() => {
      value1v = 101
      return value1a
    })
    expect(value1v).toBe(101)
    // check that writers cannot run
    let value2 = 0
    const [wait2, set2] = mkPromise<number>()
    const prom2 = lock.writeLock(() => {
      value2 = 1
      return wait2
    })
    expect(lock.state()).toBe('read')
    expect(value2).toBe(0)
    // check that writer will not be triggered by only one reader completing
    set1a(102)
    expect(await prom1a).toBe(102)
    expect(lock.state()).toBe('read')
    expect(value2).toBe(0)
    // check that further readers are now blocked
    let value3 = 10
    const prom3 = lock.readLock(() => (value3 = 11))
    expect(lock.state()).toBe('read')
    expect(value3).toBe(10)
    // now check that the writer is first unblocked
    set1(2)
    expect(await prom1).toBe(2)
    await delay() // to give the prom2 task a chance to make progress
    expect(value2).toBe(1)
    expect(value3).toBe(10)
    // and finally the reader runs
    set2(2)
    expect(await prom2).toBe(2)
    await prom3
    expect(value3).toBe(11)
  })
})
