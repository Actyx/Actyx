/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */

const objectOverhead = 24

// https://stackoverflow.com/questions/40512393/understanding-string-heap-size-in-javascript-v8
const stringSize = (text: string): number => objectOverhead + Math.ceil(text.length / 8)

/**
 * Roughly estimate size of an object
 * @param obj the object to measure
 * @param internedStrings true if strings should be counted only once
 */
export const sizeof = (obj: any, internedStrings: boolean = false): number => {
  let size: number = 0
  const sm = new Map<string, string>()
  const om = new Map<any, any>()
  const size0 = (o: any): void => {
    switch (typeof o) {
      case 'number':
        // assume number is stored unboxed
        size += 8
        break
      case 'string':
        // size of the pointer to the string
        size += 8
        if (internedStrings) {
          // count every string only once
          if (!sm.has(o)) {
            // 1 byte per char, plus 24 bytes object overhead
            size += stringSize(o)
            sm.set(o, o)
          }
        } else {
          // 1 byte per char, plus 24 bytes object overhead
          size += stringSize(o)
        }
        break
      case 'object':
        // size of the pointer to the object
        size += 8
        if (o !== null && !om.has(o)) {
          // we don't count keys because we hope that hidden classes will usually take care of them
          Object.values(o).forEach(size0)
          // object overhead
          size += objectOverhead
          // remember this object
          om.set(o, o)
        }
        break
      case 'boolean':
        // supposedly bools take only 4 bytes
        size += 4
        break
      case 'symbol':
        // pointer to symbol
        size += 8
        break
      case 'undefined':
        // pointer to undefined
        size += 8
        break
      case 'function':
        // pointer to function
        size += 8
        break
    }
  }
  size0(obj)
  return size
}
