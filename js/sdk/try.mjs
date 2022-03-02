import { Actyx } from './lib'

const run = async () => {
  const a = await Actyx.of(
    { appId: 'com.example.x', displayName: 'test', version: '1' },
    { actyxPort: Number(process.argv[2]) || 4454 },
  )
  const i = await a.nodeInfo(0)
  console.log('long:', i.longVersion())
  console.log('semver:', i.semVer())
  console.log('uptime:', i.uptimeMillis())
  console.log('conns:', i.connectedNodes())
  a.dispose()
}
run()
