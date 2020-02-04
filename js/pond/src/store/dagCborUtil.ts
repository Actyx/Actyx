/* eslint-disable @typescript-eslint/no-explicit-any */

import * as cbor from 'borc'
import * as CID from 'cids'
import * as dagCbor from 'ipld-dag-cbor'
import { Observable } from 'rxjs'
import { Link } from '../util'

const CID_CBOR_TAG = 42

const sanitizeInPlace = (value: any): void => {
  if (value !== null && typeof value === 'object') {
    if (Array.isArray(value)) {
      for (let i = 0; i < value.length; i++) {
        const v = value[i]
        if (v === undefined) {
          value[i] = null
        } else {
          sanitizeInPlace(v)
        }
      }
    } else {
      Object.entries(value).forEach(([k, v]) => {
        if (v === undefined) {
          delete value[k]
        } else {
          sanitizeInPlace(v)
        }
      })
    }
  }
}

const decoder = new cbor.Decoder({
  tags: {
    [CID_CBOR_TAG]: (val: Buffer) => {
      // remove that 0
      const val1 = val.slice(1)
      const text = new CID(val1).toBaseEncodedString()
      return Link.of(text)
    },
  },
  size: 4000000,
})

const encode = <T>(value: T): Observable<Buffer> =>
  new Observable<Buffer>(subscriber => {
    sanitizeInPlace(value)
    dagCbor.util.serialize(value, (err: any, serialized: Buffer) => {
      if (err !== null) {
        subscriber.error(err)
      } else {
        subscriber.next(serialized)
        subscriber.complete()
      }
    })
  })

const decode = (buffer: Buffer): Observable<any> =>
  new Observable<any>(s => {
    try {
      const decoded = decoder.decodeFirst(buffer)
      // remove once we are certain that there can never be undefined coming from decode
      // we can do this in place since we own the object. After all we just created it.
      sanitizeInPlace(decoded)
      s.next(decoded)
      s.complete()
    } catch (err) {
      s.error(err)
    }
  })

export const DagCbor = {
  encode,
  decode,
}
