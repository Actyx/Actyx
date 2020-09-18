export type RwLockState = 'read' | 'write' | 'idle'

export class RwLock {
  /**
   * rw > 0: readers
   * rw = 0: idle
   * rw = -1: writer
   */
  private rw = 0
  /** tasks wait list */
  private queue: (() => void)[] = []

  readLock = async <T>(f: () => Promise<T> | T): Promise<T> => {
    const release = (t: T) => {
      this.rw -= 1
      if (this.queue.length > 0) {
        this.queue[0]()
      }
      return t
    }
    if (this.rw >= 0 && this.queue.length == 0) {
      this.rw += 1
      return Promise.resolve(f()).then(release)
    } else {
      return new Promise((res) =>
        this.queue.push(() => {
          if (this.rw >= 0) {
            this.queue.shift()
            this.rw += 1
            setTimeout(() => Promise.resolve(f()).then(release).then(res))
          }
        }),
      )
    }
  }

  writeLock = async <T>(f: () => Promise<T> | T): Promise<T> => {
    const release = (t: T) => {
      this.rw = 0
      if (this.queue.length > 0) {
        this.queue[0]()
      }
      return t
    }
    if (this.rw == 0 && this.queue.length == 0) {
      this.rw = -1
      return Promise.resolve(f()).then(release)
    } else {
      return new Promise((res) =>
        this.queue.push(() => {
          if (this.rw == 0) {
            this.queue.shift()
            this.rw = -1
            setTimeout(() => Promise.resolve(f()).then(release).then(res))
          }
        }),
      )
    }
  }

  state = (): RwLockState => (this.rw > 0 ? 'read' : this.rw < 0 ? 'write' : 'idle')
}
