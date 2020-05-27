/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { CronJob } from 'cron'
import { Observable } from 'rxjs'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const cron = (cronTime: string): Observable<any> =>
  new Observable<void>(subscriber => {
    const job = new CronJob({
      cronTime,
      onTick: () => {
        subscriber.next(undefined)
      },
      onComplete: () => {
        subscriber.complete()
      },
      start: true,
    })
    return () => {
      job.stop()
    }
  })
