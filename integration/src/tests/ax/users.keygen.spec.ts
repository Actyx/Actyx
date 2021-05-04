import { runOnEach } from '../../infrastructure/hosts'

describe('ax', () => {
  describe('users keygen', () => {
    it('should work on Windows', async () => {
      await runOnEach([{ os: 'windows' }], async (node) => {
        const response = await node.target.execute(String.raw`
        $tempFolderPath = Join-Path $Env:Temp $(New-Guid)
        New-Item -Type Directory -Path $tempFolderPath | Out-Null
        $out = Join-Path $tempFolderPath id
        Start-Process -Wait -FilePath C:\Users\Administrator\AppData\Local\Actyx\ax.exe -ArgumentList 'users keygen --output',$out -RedirectStandardOutput stdout.txt
        Get-Content stdout.txt
        Get-Content $out
        $pub = Join-Path $tempFolderPath id.pub
        Get-Content $pub`)
        expect(response.exitCode).toBe(0)
        expect(response.stdOut.startsWith('Your private key has been saved at')).toBeTruthy()
        expect(response.stdOut.split('\n').length).toBe(5)
        expect(response.stdErr).toBe('Generating public/private key pair ..')
      })
    })
  })
})
