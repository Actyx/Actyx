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
  const actyx = await Actyx.of(
    {
      appId: 'com.example.dev-pond',
      displayName: 'Pond dev',
      version: '1.0.0',
    },
    {
      automaticReconnect: true,
    },
  ).catch(ex => {
    console.log('cannot start SDK, is Actyx running on this computer?', ex)
    process.exit(1)
  })

  console.log('Hello')

  const tags3 = Tags('tE')

  const p = new Observable(o =>
    actyx.observeLatest({ query: tags3 }, e => o.next(e), err => o.error(err)),
  )

  console.log(await actyx.publish(tags3.apply('x')))

  console.log('waiting for err (stop the store manually)')

  try {
    await p.toPromise()
  } catch (ex) {
    console.log('Caught', ex)
  }

  console.log('waiting a while for you to restart the store')
  await new Promise(resolve => setTimeout(resolve, 20000))

  console.log('trying to send another request')

  console.log(await actyx.publish(tags3.apply('qqqq')))
}

start()
