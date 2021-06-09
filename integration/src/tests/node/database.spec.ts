import { runOnEvery } from '../../infrastructure/hosts'
import { settings } from '../../infrastructure/settings'
import { randomString, runActyx, runActyxVersion, runUntil } from '../../util'

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
      const outV1 = await runUntil(v1, 'db-1.1.5', ['ActyxOS started.'], 10_000)
      expect(outV1).toContainEqual(expect.stringContaining('ActyxOS started.'))

      // now run current version to check error message
      const current = await runUntil(runActyx(node, workdir), 'db-current', [], 5_000)
      if (Array.isArray(current)) {
        throw new Error(`timed out:\n${current.join('\n')}`)
      }

      const template = String.raw`using data directory ${'`.*' + main + '`'}
        .*
        Attempting to start Actyx v2 with a data directory from ActyxOS v1\.1, which is currently not supported\.
        See the documentation for when and how migration is supported\.
        Meanwhile, you can start from a fresh data directory \(see also the --working-dir command line option\)\.`
      const regex = new RegExp(template.replace(/\s+/g, ' '))
      expect(current.stderr.replace(/\s+/g, ' ')).toMatch(regex)
    }))
})
