/**
 * @jest-environment ./dist/jest/environment
 */
/* eslint-disable no-empty */
import { execa, execaCommand } from 'execa'
import { settings } from '../../infrastructure/settings'
import { randomString, runUntil } from '../../util'

describe('v1 to v2 migration', () => {
  it('should work on Docker', async () => {
    try {
      await execa('docker', ['ps'])
    } catch (e) {
      console.error('No local docker daemon available. Skipping.')
      return
    }
    const volumeId = randomString()
    let v1container
    let v2container
    await execa('docker', ['volume', 'create', volumeId])
    try {
      v1container = (
        await execa('docker', [
          'run',
          '-d',
          '--privileged',
          '-e',
          'ENABLE_DEBUG_LOGS=1',
          '-v',
          `${volumeId}:/data`,
          'actyx/os:1.1.5',
        ])
      ).stdout
      expect(
        await runUntil(
          Promise.resolve({
            process: execa('docker', ['logs', '-f', v1container]),
            workdir: volumeId,
          }),
          'migration.spec',
          ['ActyxOS ready'],
          20000,
        ),
      ).toContainEqual(expect.stringContaining('ActyxOS ready'))
      // Crash the container
      await execa('docker', ['container', 'rm', '-f', v1container])
      v1container = undefined

      // fix permissions
      await execa('docker', [
        'run',
        '--rm',
        '-v',
        `${volumeId}:/data`,
        '-u',
        'root',
        'alpine',
        'chown',
        '-R',
        '1000:1000',
        '/data',
      ])
      v2container = (
        await execa('docker', [
          'run',
          '-d',
          '-v',
          `${volumeId}:/data`,
          `actyx/actyx-ci:actyx-${settings().gitHash || (await currentHead())}`,
        ])
      ).stdout
      const v2out = await runUntil(
        Promise.resolve({
          process: execa('docker', ['logs', '-f', v2container]),

          workdir: volumeId,
        }),
        'migration.spec',
        ['NODE_STARTED_BY_HOST'],
        20000,
      )
      expect(v2out).toContainEqual(expect.stringContaining('Migration succeeded.'))
      expect(v2out).toContainEqual(expect.stringContaining('NODE_STARTED_BY_HOST'))
    } finally {
      try {
        v1container && (await execa('docker', ['container', 'rm', '-f', v1container]))
      } catch (_) {}
      try {
        v2container && (await execa('docker', ['container', 'rm', '-f', v2container]))
      } catch (_) {}
      await execa('docker', ['volume', 'rm', '-f', volumeId])
    }
  })
})

const currentHead = () => execaCommand('git rev-parse HEAD').then((x) => x.stdout)
