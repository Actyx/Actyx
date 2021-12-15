/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */

import { Offset } from './various'
import { _offsetMapLookup } from '.'

describe('util', () => {
  it('should be able to lookup in offset map', () => {
    expect(_offsetMapLookup({ a: 999 }, 'a')).toBe(999)
    expect(_offsetMapLookup({ 'a-b-c': -999 }, 'a-b-c')).toBe(-999)
    expect(_offsetMapLookup({ a: 0 }, 'a-b-c')).toBe(Offset.min)
  })
})
