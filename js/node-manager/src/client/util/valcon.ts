import { Obs } from './obs'

/**
 * Simple value container
 */
export type Valcon<T extends unknown> = {
  get: () => T
  set: (t: T) => unknown
}
export const Valcon = <T>(t: T): Valcon<T> => {
  let val = t
  return {
    get: () => val,
    set: (newval: T) => {
      val = newval
    },
  }
}

export type ObsValcon<T extends unknown> = Valcon<T> & {
  obs: Obs<T>
}

/**
 * Observable value container
 */
export const ObsValcon = <T extends unknown>(t: T) => {
  const valcon = Valcon(t)
  const obs = Obs.make<T>()
  const set: ObsValcon<T>['set'] = (newval: T) => {
    valcon.set(newval)
    obs.emit(newval)
  }
  return {
    ...valcon,
    obs,
    set,
  }
}
