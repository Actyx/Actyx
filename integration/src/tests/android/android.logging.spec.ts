import { Observable } from 'rxjs'
import { assertOK } from '../../assertOK'
import { LogLine } from '../../cli'
import { runOnEach } from '../../infrastructure/hosts'

describe('Logging on Android', () => {
  test('should write stuff to logcat', async () => {
    await runOnEach([{ host: 'android' }], async () => {
      // qed by having a running android node
    })
  })
  test('should read stuff from logcat', async () => {
    await runOnEach([{ host: 'android' }], async (node) => {
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const execAdb = (s: string) => node.target.executeInContainer!(s, [])
      // Trace is not exposed via ax settings (??)
      const logLevels = [
        ['d', 'DEBUG'],
        ['i', 'INFO'],
        ['w', 'WARN'],
        ['e', 'ERROR'],
        ['f', 'ERROR'],
      ]
      const tags = ['some_tag', 'some.other.tag', 'com.actyx.android.weird']
      // Can't use `adb shell run-as com.actyx.android` as the APK does not have
      // the `debuggable` flag set in release mode. So we work around this by
      // using `su`, meaning that this only works on devices where root is
      // available. Fortunately the emulator provides `su`.
      const uid = (await execAdb('shell pm list packages -U | grep com.actyx.android')).stdout
        // Sample output: package:com.actyx.android uid:10077
        .split(' ')[1]
        .split(':')[1]

      const matchers = await Promise.all(
        logLevels.flatMap(([androidLevel, actyxLevel]) =>
          tags.map(async (tag) => {
            const msg = 'w00t 1 2 3'
            await execAdb(`shell su ${uid} log -p ${androidLevel} -t ${tag} ${msg}`)
            return (line: LogLine) =>
              line.severity === actyxLevel &&
              line.producerName.startsWith('android.logcat') &&
              line.logName.includes(tag) &&
              line.message === msg
          }),
        ),
      )
      await Observable.timer(5000).first().toPromise()
      const logs = await node.ax.logs
        .tail(true, 20, false)
        .map(assertOK)
        .flatMap((x) => x.result)
        .toArray()
        .toPromise()

      console.log(`Got back ${logs.length} logs`)
      matchers.forEach((m) => expect(logs.findIndex((entry) => m(entry)) >= 0).toBeTruthy())
    })
  })
})
