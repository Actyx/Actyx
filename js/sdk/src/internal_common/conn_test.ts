/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2020 Actyx AG
 */
import { Observable } from 'rxjs'
import { Actyx, Tags } from '..'

// Just a manual test that connects to live Actyx store, to test stuff with quick turnaround
const start = async () => {
  const actyx = await Actyx.of({
    appId: 'com.example.dev-pond',
    displayName: 'Pond dev',
    version: '1.0.0',
  }).catch(ex => {
    console.log('cannot start SDK, is Actyx running on this computer?', ex)
    process.exit(1)
  })

  const tags3 = Tags('tE')

  const p = new Observable(o => actyx.observeLatest({ query: tags3 }, e => o.next(e)))

  actyx.publish(tags3.apply('x'))

  const d = await p.take(1).toPromise()

  console.log(d)
}

start()
