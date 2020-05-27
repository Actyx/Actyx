/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { cron } from './cron'

describe('cron', () => {
  it(
    'should allow running in regular intervals',
    async () => {
      const result = await cron('* * * * * *')
        .take(10)
        .map(() => Date.now() / 1000) // seconds
        .toArray()
        .toPromise()
      // test that the difference between invocations is about 1s
      result.forEach((t1, i) => {
        if (i > 0) {
          const t0 = result[i - 1]
          const delta = t1 - t0
          expect(delta).toBeLessThan(2) // Fails on CI if we assert closeTo(1)
        }
      })
      // test that the absolute time of an invocation is roughly at a whole second
      result.forEach(t => {
        expect(Math.abs(t % 1)).toBeLessThan(0.7)
      })
    },
    20000,
  )
})
