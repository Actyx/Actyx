import { contramap, Ord } from 'fp-ts/lib/Ord'
import { Ordering, sign } from 'fp-ts/lib/Ordering'
import { setoidString } from 'fp-ts/lib/Setoid'

type Locale = 'en' | 'de' | 'es' | 'pl' | 'cs'

export const alphaNumCompare = (locale: Locale) => (a: string, b: string): Ordering =>
  sign(a.localeCompare(b, locale, { numeric: true }))

// This warrants some discussion, the optimization leads to a minor speed increase when sorting long lists but I'm not exactly sure how bad
// the consequences of this less accurate sorting would be
export const ordStringLocale = <A>(f: (a: A) => string) => (_locale: Locale): Ord<A> => ({
  compare: (a, b) => {
    const aa = f(a)
    const bb = f(b)
    if (aa === bb) {
      return 0
    } else if (aa > bb) {
      return 1
    } else {
      return -1
    }
  },
  equals: (a, b) => f(a) === f(b),
})

export const ordStringLocaleOld = <A>(f: (a: A) => string) => (locale: Locale): Ord<A> =>
  contramap(f, {
    ...setoidString,
    compare: alphaNumCompare(locale),
  })
