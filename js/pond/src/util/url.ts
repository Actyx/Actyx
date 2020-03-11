/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { compose, filter, head, last, map, reduce, split } from 'ramda'

interface QueryParams {
  [key: string]: string | true | undefined
}

// workaround https://github.com/types/npm-ramda/pull/326
const mapSplit = (v: ReadonlyArray<string>) => map(split('='), v)

export const parseQueryStringIntoObject = compose(
  x =>
    reduce(
      (acc, v) => {
        acc[v[0]] = v[1] || true
        return acc
      },
      {} as QueryParams,
      x,
    ),
  mapSplit,
  split('&'),
)

export const getUrlQueryParams: (_: string) => QueryParams = compose(
  // eslint-disable-next-line @typescript-eslint/ban-ts-ignore
  // @ts-ignore
  x => (x.length === 2 ? parseQueryStringIntoObject(last(x)) : {}),
  filter(Boolean),
  split('?'),
)

export const getUrlHashValue: (_: string) => string = compose(
  (x: string[]) => (x.length === 2 ? (last(x) as string) : ''),
  split('#'),
  head,
  split('?'),
)
