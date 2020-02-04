// there is something similar in ramda (basically map), but the types don't work.
// See https://github.com/types/npm-ramda/issues/311
// In any case, stuff in ramda is notoriously slow.
export const mapValues = <A, B>(
  x: Readonly<{ [key: string]: A }>,
  f: (value: A) => B,
): { [key: string]: B } => {
  const r: { [key: string]: B } = {}
  Object.entries(x).forEach(([k, v]) => {
    r[k] = f(v)
  })
  return r
}
