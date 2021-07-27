import { readFileSync } from 'fs-extra'
import { assertOK } from '../../assertOK'
import { CLI } from '../../cli'
import { SettingsInput } from '../../cli/exec'
import { runOnEach, runOnEvery } from '../../infrastructure/hosts'
import { currentAxBinary } from '../../infrastructure/settings'

describe('ax', () => {
  describe('users keygen', () => {
    it('should work on Windows', async () => {
      await runOnEach([{ os: 'windows' }], async (node) => {
        const response = await node.target.execute(
          String.raw`
        $tempFolderPath = Join-Path $Env:Temp $(New-Guid)
        New-Item -Type Directory -Path $tempFolderPath | Out-Null
        $out = Join-Path $tempFolderPath id
        Start-Process -Wait -FilePath C:\ax.exe -ArgumentList 'users keygen --output',$out -RedirectStandardOutput stdout.txt
        Get-Content stdout.txt
        Get-Content $out
        $pub = Join-Path $tempFolderPath id.pub
        Get-Content $pub`,
          [],
        ).process
        expect(response.exitCode).toBe(0)
        expect(response.stdout.startsWith('Your private key has been saved at')).toBeTruthy()
        expect(response.stdout.split('\n').length).toBe(5)
        expect(response.stderr).toBe('Generating public/private key pair ..')
      })
    })
  })
})

describe('authorizing users', () => {
  test('add and remove an additional user', async () => {
    await runOnEvery(async (node) => {
      // This will generate a CLI with a different than private key the node
      // was setup with
      const secondCli = await CLI.build(
        `${node._private.hostname}:${node._private.adminPort}`,
        await currentAxBinary(),
      )
      const err = assertOK(await secondCli.nodes.ls())
      expect(err.result[0].connection).toBe('unauthorized')

      const key = readFileSync(secondCli.identityPath + '.pub')
        .toString('utf8')
        .trim()
      const scope = '/admin/authorizedUsers'
      const existing: string[] = assertOK(await node.ax.settings.get(scope)).result as string[]

      const authorizedUsers = existing.concat([key])
      assertOK(await node.ax.settings.set(scope, SettingsInput.FromValue(authorizedUsers)))

      const ok = assertOK(await secondCli.nodes.ls())
      expect(ok.result[0].connection).toBe('reachable')

      // Remove the user again
      assertOK(await node.ax.settings.set(scope, SettingsInput.FromValue(existing)))

      const err0 = assertOK(await secondCli.nodes.ls())
      expect(err0.result[0].connection).toBe('unauthorized')
    })
  })
})
