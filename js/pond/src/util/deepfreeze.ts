// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function deepFreeze<T extends any>(o: T): T {
  Object.freeze(o)

  Object.getOwnPropertyNames(o).forEach(prop => {
    if (
      // eslint-disable-next-line no-prototype-builtins
      o.hasOwnProperty(prop) &&
      o[prop] !== null &&
      (typeof o[prop] === 'object' || typeof o[prop] === 'function') &&
      !Object.isFrozen(o[prop])
    ) {
      deepFreeze(o[prop])
    }
  })

  return o
}
