/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import * as shajs from 'sha.js'

/**
 * An IPFS DAG object can be represented as an arbitrary JSON object (including a primitive).
 * For details on the mapping to the internal representation, see https://github.com/ipld/specs/tree/master/ipld
 */
export type DagObject = any

/*
This method is intended as an augmented replacement of Ramda's pathOr for the particular
use case of traversing Ipfs dag. It works with the standard case of dag encoded in-line
like this: {a : { b : 1}} (so like pathOr) but also in case of dag encoded like this
{a : {"/" : "hashOfB"}}, which is one of the ipld encodings. See testCommandExecutor.spec.ts
for examples of usage.
This method will resolve all the hashes it encounters as values for key '/', including the last one,
in order to mimic as good as possible ipfs dag get. In case the last hash cannot be resolved it will
return defaultValue.
*/
export const augmentedPathOr = (p: ReadonlyArray<string>, dag: any, defaultValue?: any) => {
  let currDag = dag
  for (let i = 0; i < p.length; i++) {
    let dagP = currDag[p[i]]
    if (dagP === undefined) {
      return defaultValue
    }
    if (dagP['/'] !== undefined) {
      dagP = dag[dagP['/']]
      if (dagP === undefined) {
        return defaultValue
      }
    }
    currDag = dagP
  }
  return currDag
}

export const putToDag: (dag: any, value: DagObject) => [any, any] = (dag, value) => {
  const raw = JSON.stringify(value)
  const hash = shajs('sha256')
    .update(raw)
    .digest('hex')

  dag[hash] = value
  return [dag, hash]
}
