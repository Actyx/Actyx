import { actyxCliWindowsBinary, currentAxBinary, windowsActyxInstaller } from './settings'
import { Ssh } from './ssh'
import { connectSsh, execSsh } from './linux'
import { ActyxNode, printTarget, SshAble, Target } from './types'
import { CLI } from '../cli'

export const mkWindowsSsh = async (
  nodeName: string,
  target: Target,
  sshParams: SshAble,
  // FIXME we currently don’t get logs for windows in CI
  logger: (s: string) => void = console.log,
): Promise<ActyxNode> => {
  console.log('setting up Actyx process: %s on %o', nodeName, printTarget(target))

  const ssh = Ssh.new(sshParams.host, sshParams.username, sshParams.privateKey)
  // Takes about 300 secs for ssh to be reachable
  await connectSsh(ssh, nodeName, sshParams, 150)

  const hereInstallerPath = await windowsActyxInstaller(target.arch)
  const thereInstallerPath = String.raw`C:\installer.msi`
  console.log(`${nodeName}: Copying ${hereInstallerPath} ${thereInstallerPath}`)
  await ssh.scp(hereInstallerPath, thereInstallerPath)
  const hereCliPath = await actyxCliWindowsBinary('x86_64')
  const thereCliPath = String.raw`C:\ax.exe`
  console.log(`${nodeName}: Copying ${hereCliPath} ${thereCliPath}`)
  await ssh.scp(hereCliPath, thereCliPath)

  console.log(`${nodeName}: Installing ${thereInstallerPath}`)
  console.log(`${nodeName}: Installing Actyx`)
  await execSsh(ssh)(
    String.raw`(Start-Process "msiexec.exe" -ArgumentList '/i ${thereInstallerPath} /qn /Liwearucmov*x C:\actyx-install.log' -NoNewWindow -Wait -PassThru).ExitCode`,
  )
  await execSsh(ssh)(String.raw`Start-Sleep -Seconds 5`)

  const defaultExeLocation = String.raw`C:\PROGRA~1\Actyx\Node\actyx.exe`
  const workingDir = String.raw`C:\PROGRA~1\Actyx\Node\actyx-data`
  const node = await forwardPortsAndBuildClients(
    thereInstallerPath,
    ssh,
    nodeName,
    target,
    workingDir,
    {
      host: 'process',
    },
  )
  return { ...node, _private: { ...node._private, actyxBinaryPath: defaultExeLocation } }
}

export const forwardPortsAndBuildClients = async (
  installerPath: string,
  ssh: Ssh,
  nodeName: string,
  target: Target,
  workingDir: string,
  theRest: Omit<ActyxNode, 'ax' | '_private' | 'name' | 'target'>,
): Promise<ActyxNode> => {
  const [[port4454, port4458], proc] = await ssh.forwardPorts(4454, 4458)

  console.log('node %s admin reachable on port %i', nodeName, port4458)
  console.log('node %s http api reachable on port %i', nodeName, port4454)

  const axBinaryPath = await currentAxBinary()
  const axHost = `localhost:${port4458}`
  console.log(`axHost: ${axHost}`)
  console.error('created cli w/ ', axHost)
  const ax = await CLI.build(axHost, axBinaryPath)

  const httpApiOrigin = `http://localhost:${port4454}`
  console.log(`httpApiOrigin: ${httpApiOrigin}`)

  const apiPond = `ws://localhost:${port4454}/api/v2/events`
  console.log(`apiPond: ${apiPond}`)

  const shutdown = async () => {
    await target._private.cleanup()
    proc.kill('SIGTERM')
  }

  const result: ActyxNode = {
    name: nodeName,
    target,
    ax,
    _private: {
      shutdown,
      actyxBinaryPath: './actyx',
      workingDir,
      axBinaryPath,
      axHost,
      httpApiOrigin,
      apiPond,
      apiSwarmPort: 4001,
      apiEventsPort: port4454,
    },
    ...theRest,
  }

  return result
}

export const mkCmd = (exe: string, params: string[]): string =>
  String.raw`Start-Process -Wait -NoNewWindow -FilePath ${exe} -ArgumentList ${params
    .concat(['--background'])
    .map((x) => `'${x}'`)
    .join(',')}`

// Create a PowerShell script which enables OpenSSH and adds `pubKey` to
// `authorized_keys`
// https://www.mirantis.com/blog/today-i-learned-how-to-enable-ssh-with-keypair-login-on-windows-server-2019/
export function makeWindowsInstallScript(pubKey: string): string {
  return String.raw`<powershell>
          Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0
          Set-Service -Name sshd -StartupType ‘Automatic’
          Start-Service sshd
          $key = "${pubKey}"
          $key | Set-Content C:\ProgramData\ssh\administrators_authorized_keys
          $acl = Get-Acl C:\ProgramData\ssh\administrators_authorized_keys
          $acl.SetAccessRuleProtection($true, $false)
          $acl.Access | %{$acl.RemoveAccessRule($_)} # strip everything
          $administratorRule = New-Object system.security.accesscontrol.filesystemaccessrule("Administrator","FullControl","Allow")
          $acl.SetAccessRule($administratorRule)
          $administratorsRule = New-Object system.security.accesscontrol.filesystemaccessrule("Administrators","FullControl","Allow")
          $acl.SetAccessRule($administratorsRule)
          (Get-Item 'C:\ProgramData\ssh\administrators_authorized_keys').SetAccessControl($acl)
          New-ItemProperty -Path "HKLM:\SOFTWARE\OpenSSH" -Name DefaultShell -Value "C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe" -PropertyType String -Force
          restart-service sshd
          </powershell>`
}
