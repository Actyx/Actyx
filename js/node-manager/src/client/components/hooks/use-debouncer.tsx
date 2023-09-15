import { useEffect, useMemo } from 'react'

export const makeDebouncer = () => {
  let storedTimeout: number | undefined = undefined

  const clear = () => clearTimeout(storedTimeout)

  const register = (fn: Function, timeout: number) => {
    clear()
    storedTimeout = setTimeout(fn, timeout) as unknown as number
  }

  return { register, clear }
}

export const useDebouncer = () => {
  const inner = useMemo(makeDebouncer, [])

  useEffect(() => {
    // clear when exit
    return () => {
      inner.clear()
    }
  }, [inner])

  return inner
}
