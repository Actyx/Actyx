export type Destruction = {
  isDestroyed: () => boolean
  destroy: () => unknown[] | undefined
  addHook: (fn: () => unknown) => unknown
}

// eslint-disable-next-line @typescript-eslint/no-redeclare
// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace Destruction {
  export const make = (): Destruction => {
    let destroyed = false
    let hooks: Function[] = []

    return {
      isDestroyed: () => destroyed,
      addHook: (hook: () => unknown) => {
        if (destroyed) return
        hooks.push(hook)
      },
      destroy: () => {
        if (destroyed) return
        destroyed = true
        const results = hooks.map((hook) => hook())
        hooks = []
        return results
      },
    }
  }
}
