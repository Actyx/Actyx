type Listener<T> = (message: T) => unknown

export type Obs<T extends any> = {
  sub: (listener: Listener<T>) => () => unknown
  unsub: (listener: Listener<T>) => void
  emit: (t: T) => unknown[]
  size: () => number
}

// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace Obs {
  export const make = <T extends any>(): Obs<T> => {
    const set = new Set<Listener<T>>()

    const unsub: Obs<T>['unsub'] = (listener) => {
      set.delete(listener)
    }

    const sub: Obs<T>['sub'] = (listener) => {
      set.add(listener)
      return () => unsub(listener)
    }

    const emit: Obs<T>['emit'] = (data) => {
      const res = Array.from(set).map((listener) => {
        try {
          return listener(data)
        } catch (err) {
          console.error(err)
          return
        }
      })
      return res
    }
    const size = () => set.size

    return {
      sub,
      unsub,
      emit,
      size,
    }
  }
}
