import { runOnEvery } from '../../infrastructure/hosts'
import { mkProcessLogger } from '../../infrastructure/mkProcessLogger'
import { settings } from '../../infrastructure/settings'
import { randomString, runActyx, runActyxVersion } from '../../util'

describe('node.sqlite', () => {
  it('should yell when using a v1 workdir', () =>
    runOnEvery(async (node) => {
      if (node.host !== 'process' || node.target.os === 'macos') {
        return
      }

      const prefix = node.target.kind.type === 'local' ? `${settings().tempDir}/` : ''
      const main = `workdir-1-1-5-${randomString()}` // this is used as a RegExp below!
      const workdir = prefix + main

      // run v1.1.5 to create an old workdir
      const [v1] = await runActyxVersion(node, '1.1.5', workdir)
      const logs: string[] = []
      try {
        await new Promise((res, rej) => {
          setTimeout(() => rej(new Error('timed out')), 10_000)
          const { log, flush } = mkProcessLogger((s) => logs.push(s), 'database-1.1.5', [
            'ActyxOS started.',
          ])
          v1.stdout?.on('data', (buf) => {
            if (log('stdout', buf)) {
              res()
            }
          })
          v1.stderr?.on('data', (buf) => log('stderr', buf))
          v1.on('close', (code, signal) => {
            flush()
            rej(new Error(`exited with code ${code} / signal ${signal}`))
          })
        })
      } catch (e) {
        console.log(logs.join('\n'))
        throw e
      } finally {
        v1.cancel()
      }

      // now run current version to check error message
      const current = await runActyx(node, workdir).catch((e) => e)
      const template = String.raw`using data directory ${'`.*' + main + '`'}
        .*
        Attempting to start Actyx v2 with a data directory from ActyxOS v1\.1, which is currently not supported\.
        See the documentation for when and how migration is supported\.
        Meanwhile, you can start from a fresh data directory \(see also the --working-dir command line option\)\.`
      const regex = new RegExp(template.replace(/\s+/g, ' '))
      expect(current.stderr.replace(/\s+/g, ' ')).toMatch(regex)
    }))
})
