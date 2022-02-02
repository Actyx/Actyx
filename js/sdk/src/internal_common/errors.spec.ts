/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */

import { decorateEConnRefused } from './errors'

describe('Error utility to decorate EConnRefused', () => {
  it(`should work with strings`, () => {
    expect(() => decorateEConnRefused('this is a string', 'this is also a string')).not.toThrow()
  })

  it(`should not throw with other types`, () => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    expect(() => decorateEConnRefused(true, 'this is also a string')).not.toThrow()
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    expect(() => decorateEConnRefused({}, 'this is also a string')).not.toThrow()
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    expect(() => decorateEConnRefused(null, 'this is also a string')).not.toThrow()
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    expect(() => decorateEConnRefused(undefined, 'this is also a string')).not.toThrow()
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    expect(() => decorateEConnRefused('this is a string', true)).not.toThrow()
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    expect(() => decorateEConnRefused('this is a string', {})).not.toThrow()
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    expect(() => decorateEConnRefused('this is a string', null)).not.toThrow()
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    expect(() => decorateEConnRefused('this is a string', undefined)).not.toThrow()
  })
})
