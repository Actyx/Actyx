export type ContainerCell<T extends Set<any> | Map<any, any>> = {
  access: () => Readonly<T>
  mutate: <R>(fn: (set: T) => R) => R
}

export const ContainerCell = <T extends Set<any> | Map<any, any>>(t: T) => {
  const proto: any = t instanceof Set ? Set : Map
  let inner: T = new proto(t) as T
  const self: ContainerCell<T> = {
    access: () => inner,
    mutate: (fn) => {
      inner = new proto(inner)
      return fn(inner)
    },
  }
  return self
}

const ContainerCellLazyCalcUninit: unique symbol = Symbol()
export const ContainerCellLazyCalc = <T extends Set<any> | Map<any, any>, R extends any>(
  cell: ContainerCell<T>,
  fn: (t: Readonly<T>) => R,
): (() => R) => {
  let lastInnerState = cell.access()
  let cache = ContainerCellLazyCalcUninit as typeof ContainerCellLazyCalcUninit | R

  return () => {
    if (cell.access() !== lastInnerState) {
      lastInnerState = cell.access()
      cache = fn(cell.access())
    }
    const ret = cache !== ContainerCellLazyCalcUninit ? cache : (fn(cell.access()) as R)
    cache = ret
    return ret
  }
}
