import { Actyx, Tags } from './lib/index.js'
import { execSync } from 'child_process'
;(async () => {
  const a = await Actyx.of({ appId: 'com.example.x', displayName: 'test', version: '42' })
  const lowerBound = await a.present()
  for (let l = 0; l < 15; l++) {
    a.subscribe(
      { lowerBound },
      () => process.stdout.write('.'),
      (e) => console.log(e),
    )
  }
})()
