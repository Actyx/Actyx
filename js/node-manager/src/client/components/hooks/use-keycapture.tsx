import { useEffect } from 'react'

export const useKeydown = (
  fn: undefined | ((e: KeyboardEvent) => unknown),
  deps: unknown[] = [],
) => {
  useEffect(() => {
    if (!fn) return
    window.addEventListener('keydown', fn)
    return () => {
      window.removeEventListener('keydown', fn)
    }
  }, [fn, ...deps])
}

// Specials
// =======

export const useCtrlEnter = (fn: undefined | (() => unknown), deps: unknown[] = []) => {
  useKeydown(
    !fn
      ? undefined
      : (e) => {
          if (!e.repeat && e.code === 'Enter' && e.ctrlKey) {
            e.preventDefault()
            e.stopPropagation()
            fn()
          }
        },
    deps,
  )
}
