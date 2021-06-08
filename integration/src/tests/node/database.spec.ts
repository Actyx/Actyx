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

      const workdir = `${settings().tempDir}/workdir-1.1.5-${randomString()}`

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
      expect(current.stderr).toMatch(`using data directory \`${workdir}\``)
      expect(current.stderr).toMatch(
        `Attempting to start Actyx v2 with a data directory from ActyxOS v1.1, which is currently not supported.`,
      )
      expect(current.stderr).toMatch(
        `See the documentation for when and how migration is supported.`,
      )
      expect(current.stderr.replace(/\s+/g, ' ')).toMatch(
        `Meanwhile, you can start from a fresh data directory (see also the --working-dir command line option).`,
      )
    }))
})
