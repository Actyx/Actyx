import { runOnEvery } from '../../infrastructure/hosts'
import { settings } from '../../infrastructure/settings'
import { randomString, runActyx, runActyxVersion, runUntil } from '../../util'

jest.setTimeout(200000)

describe('node.sqlite', () => {
  // FIXME doesn't work on Windows since now a service
  it('should yell when using a v1 workdir', () =>
    runOnEvery(async (node) => {
      if (node.host !== 'process' || node.target.os === 'macos') {
        return
      }

      const prefix = node.target.kind.type === 'local' ? `${settings().tempDir}/` : ''
      const main = `workdir-1-1-5-${randomString()}` // this is used as a RegExp below!
      const workdir = prefix + main

      // run v1.1.5 to create an old workdir
      const v1Out = await runUntil(
        runActyxVersion(node, '1.1.5', workdir),
        ['ActyxOS started.'],
        10_000,
      )
      expect(v1Out).toContainEqual(expect.stringContaining('ActyxOS started.'))

      // now run current version to check error message
      const currOut = await runUntil(runActyx(node, workdir, []), ['NODE_STARTED_BY_HOST'], 10_000)
      if (Array.isArray(currOut)) {
        throw new Error(`timed out or started successfully:\n${currOut.join('\n')}`)
      }

      const template = String.raw`using data directory ${'`.*' + main + '`'}
        .*
        Attempting to start Actyx v2 with a data directory from ActyxOS v1\.1, which is currently not supported\.
        See the documentation for when and how migration is supported\.
        Meanwhile, you can start from a fresh data directory \(see also the --working-dir command line option\)\.`
      const regex = new RegExp(template.replace(/\s+/g, ' '))
      expect(currOut.stderr.replace(/\s+/g, ' ')).toMatch(regex)
    }))
})
