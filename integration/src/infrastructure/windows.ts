import { EC2 } from 'aws-sdk'
import execa from 'execa'
import { CreateEC2 } from '../../jest/types'
import { getPipEnv } from '../setup-projects'
import { instanceToTarget } from './aws'
import { forwardPortsAndBuildClients } from './linux'
import { mkProcessLogger } from './mkProcessLogger'
import { windowsActyxOsInstaller } from './settings'
import { Ssh } from './ssh'
import * as path from 'path'
import { ActyxOSNode, AwsKey } from './types'

export const mkNodeWinRM = async (
  ec2: EC2,
  prepare: CreateEC2,
  key: AwsKey,
  ciRun: string,
  nodeName: string,
  adminPW: string,
  publicKeyPath: string,
  logger: (line: string) => void,
): Promise<ActyxOSNode | undefined> => {
  const absInstallerPath = await windowsActyxOsInstaller('x86_64')
  // Input to Ansible is the relative path to a folder in which
  // `ActyxOS-Installer.exe` exists. The path needs to be relative to the sub
  // directory where the relevant ansible task lies.
  const relInstallerFolderPath = path.relative(
    'ansible/roles/prepare_windows',
    path.dirname(absInstallerPath),
  )
  const pipEnv = await getPipEnv()
  const proc = execa.command(
    `${pipEnv} run ansible-playbook -i inventory/actyx.aws_ec2.yml -v playbook.yml`,
    {
      all: true,
      cwd: 'ansible',
      env: {
        CI_RUN: ciRun,
        EC2_ADMIN_PW: adminPW,
        EC2_IMAGE_ID: prepare.ami,
        EC2_INSTANCE_TYPE: prepare.instance,
        EC2_KEY_NAME: key.keyName,
        EC2_NODE_NAME: nodeName,
        SSH_PUBLIC_KEY: publicKeyPath,
        LOCAL_INSTALLER_DIR: relInstallerFolderPath,
      },
    },
  )

  const { log, flush } = mkProcessLogger(logger, nodeName, [adminPW])

  const instance = await new Promise<EC2.Instance | undefined>((res, rej) => {
    proc.stdout?.on('data', (s: Buffer | string) => {
      const lines = log('stdout', s)
      if (lines) {
        const start = lines[0].indexOf('{')
        try {
          const obj = JSON.parse(lines[0].substring(start))
          const id = obj.instance_ids?.[0]
          if (id === undefined || typeof id !== 'string') {
            console.log('node %s weird ansible response', nodeName, obj)
            return
          }
          console.log('node %s instance started %s', nodeName, id)
          ec2.describeInstances({ InstanceIds: [id] }, (err, data) =>
            err ? rej(err) : res(data.Reservations?.[0]?.Instances?.[0]),
          )
        } catch (e) {
          // ignore
        }
      }
    })
    proc.stderr?.on('data', (s: Buffer | string) => log('stderr', s))
    proc.on('exit', () => res())
  })

  try {
    await proc
  } catch (err) {
    // logs are printed by our logger anyway
    delete err.all
    throw err
  }
  flush()

  if (instance === undefined) {
    console.log('node %s weird: didn’t see instance start', nodeName)
    return
  }

  const target = instanceToTarget(instance, prepare, key, ec2)

  const ssh = new Ssh(target.kind.host, target.kind.username, target.kind.privateKey)
  const actyxOsProc = await startActyxOS(nodeName, logger, ssh)
  return await forwardPortsAndBuildClients(ssh, nodeName, target, actyxOsProc[0], {
    host: 'process',
  })
}

function startActyxOS(
  nodeName: string,
  logger: (s: string) => void,
  ssh: Ssh,
  command = 'C:\\Users\\Administrator\\AppData\\Local\\ActyxOS\\actyx.exe --working-dir C:\\Users\\Administrator\\AppData\\Local\\ActyxOS\\actyx-data --background',
): Promise<[execa.ExecaChildProcess<string>]> {
  // awaiting a Promise<Promise<T>> yields T (WTF?!?) so we need to put it into an array
  return new Promise((res, rej) => {
    const { log, flush } = mkProcessLogger(logger, nodeName, ['NODE_STARTED_BY_HOST'])
    const proc = ssh.exec(command)
    proc.stdout?.on('data', (s: Buffer | string) => {
      if (log('stdout', s)) {
        res([proc])
      }
    })
    proc.stderr?.on('data', (s: Buffer | string) => log('stderr', s))
    proc.on('close', () => {
      flush()
      logger(`node ${nodeName} ActyxOS channel closed`)
      rej('closed')
    })
    proc.on('error', (err: Error) => {
      logger(`node ${nodeName} ActyxOS channel error: ${err}`)
      rej(err)
    })
    proc.on('exit', (code: number, signal: string) => {
      logger(`node ${nodeName} ActyxOS exited with code=${code} signal=${signal}`)
      rej('exited')
    })
  })
}
