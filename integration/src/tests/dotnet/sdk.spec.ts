import execa from 'execa'
import { runOnEvery } from '../../infrastructure/hosts'
import { dotnetIntegrationTestsAssembly } from '../../infrastructure/settings'

it('.NET SDK integration tests', () =>
  runOnEvery(async (node) => {
    const { hostname, apiPort } = node._private
    const assembly = await dotnetIntegrationTestsAssembly()
    const res = await execa('dotnet', ['test', assembly], {
      env: { AX_CLIENT_HOST: hostname, AX_CLIENT_API_PORT: `${apiPort}` },
      stdio: 'inherit',
    })
    expect(res.exitCode).toEqual(0)
  }))
