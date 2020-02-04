/* eslint-disable @typescript-eslint/no-explicit-any */
export type Cid = string
export type Link<T> = { ['/']: Cid }
export type MaybeLink<T> = Link<T> | undefined
const of = <T>(target: Cid): Link<T> => ({ '/': target })
const maybe = <T>(target?: Cid): MaybeLink<T> => (target !== undefined ? of<T>(target) : undefined)
const isLink = (x: any): x is Link<any> =>
  typeof x === 'object' && '/' in x && typeof x['/'] === 'string'
const asLink = <T>(x: any): MaybeLink<T> => (Link.isLink(x) ? Link.of<T>(x['/']) : undefined)
const getCid = <T>(x: Link<T>): Cid => x['/']
export const Link = {
  of,
  maybe,
  isLink,
  asLink,
  getCid,
}
