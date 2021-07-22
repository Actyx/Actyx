import {
  ResponseAppsSign,
  Response_Nodes_Ls,
  Response_Nodes_Inspect,
  Response_Settings_Get,
  Response_Settings_Set,
  Response_Settings_Unset,
  Response_Settings_Schema,
  Response_Swarms_Keygen,
  Response_Users_Keygen,
} from './types'
import execa from 'execa'
import * as path from 'path'
import { rightOrThrow } from '../infrastructure/rightOrThrow'
import {
  AxEventService,
  handleStreamResponse,
  mkAuthHttpClient,
  mkEventService,
  OffsetsResponse,
  PublishResponse,
  QueryResponse,
  SubscribeMonotonicResponse,
  trialManifest,
} from '../http-client'
import { dotnetEventsCliAssembly } from '../infrastructure/settings'
import { EventClients } from '../infrastructure/types'

const exec = async (binaryPath: string, args: string[], options?: execa.Options) => {
  try {
    const binaryPathResolved = path.resolve(binaryPath)
    const response = await execa(binaryPathResolved, [`-j`, ...args], options)
    return JSON.parse(response.stdout)
  } catch (error) {
    try {
      return JSON.parse(error.stdout)
    } catch (errParse) {
      console.error(error)
      throw errParse
    }
  }
}

type SettingsInput = SettingsInputFile | SettingsInputValue
type SettingsInputFile = {
  readonly key: 'SettingsInputFile'
  path: string
}

type SettingsInputValue = {
  readonly key: 'SettingsInputValue'
  value: unknown
}

export interface SettingsInputMatcher<T> {
  File: (input: SettingsInputFile) => T
  Value: (input: SettingsInputValue) => T
}

export const SettingsInput = {
  FromFile: (filePath: string): SettingsInputFile => ({ key: 'SettingsInputFile', path: filePath }),
  FromValue: (value: unknown): SettingsInputValue => ({ key: 'SettingsInputValue', value: value }),
  match: <T>(matcher: SettingsInputMatcher<T>) => (input: SettingsInput): T => {
    switch (input.key) {
      case 'SettingsInputFile':
        return matcher.File(input)
      case 'SettingsInputValue':
        return matcher.Value(input)
    }
  },
}

type Exec = {
  version: () => Promise<string>
  apps: {
    sign: (devCertFilePath: string, appManifestFilePath: string) => Promise<ResponseAppsSign>
  }
  users: {
    keyGen: (file: string) => Promise<Response_Users_Keygen>
  }
  swarms: {
    keyGen: (file?: string) => Promise<Response_Swarms_Keygen>
  }
  nodes: {
    ls: () => Promise<Response_Nodes_Ls>
    inspect: () => Promise<Response_Nodes_Inspect>
  }
  settings: {
    get: (scope: string, noDefaults?: boolean) => Promise<Response_Settings_Get>
    set: (scope: string, input: SettingsInput) => Promise<Response_Settings_Set>
    unset: (scope: string) => Promise<Response_Settings_Unset>
    schema: () => Promise<Response_Settings_Schema>
  }
  internal: {
    shutdown: () => Promise<void>
  }
}

export const mkEventClients = async (hostname: string, port: number): Promise<EventClients> => ({
  AxHttpClient: mkEventService(await mkAuthHttpClient(trialManifest)(`http://${hostname}:${port}`)),
  '.NET SDK (HTTP)': await mkDotnetEventsExec(hostname, port, false),
  '.NET SDK (Websocket)': await mkDotnetEventsExec(hostname, port, true),
})

const mkDotnetEventsExec = async (
  hostname: string,
  port: number,
  websocket?: boolean,
): Promise<AxEventService> =>
  mkEventsExec(
    'dotnet',
    [
      await dotnetEventsCliAssembly(),
      'events',
      '--manifest',
      JSON.stringify(trialManifest),
      ...(websocket ? ['--websocket'] : []),
    ],
    `${hostname}:${port}`,
  )

const mkEventsExec = (binaryPath: string, commonArgs: string[], node: string): AxEventService => {
  const run = async (cmd: string, params: string[]) => {
    const response = await execa(binaryPath, [...commonArgs, cmd, ...params])
    return JSON.parse(response.stdout)
  }
  const stream = (cmd: string, params: string[]) =>
    execa(binaryPath, [...commonArgs, cmd, ...params], {
      buffer: false,
      stdout: 'pipe',
      stderr: 'inherit',
    })

  return {
    offsets: async () => {
      const response = await run('offsets', [node])
      return rightOrThrow(OffsetsResponse.decode(response), response)
    },
    publish: async (request) => {
      const events = request.data.map((x) => `${JSON.stringify(x)}`)
      const response = await run('publish', [node, ...events])
      return rightOrThrow(PublishResponse.decode(response), response)
    },
    query: async (request, onData) => {
      const { lowerBound, upperBound, query, order } = request
      const args = [
        ...(lowerBound ? ['--lower-bound', JSON.stringify(lowerBound)] : []),
        ...(upperBound ? ['--upper-bound', JSON.stringify(upperBound)] : []),
        ...(order ? ['--order', order] : []),
        node,
        query,
      ]
      const process = stream('query', args)
      process.stdout &&
        (await handleStreamResponse(QueryResponse, onData, process.stdout, () => process.cancel()))
    },
    subscribe: async (request, onData) => {
      const { lowerBound, query } = request
      const args = [
        ...(lowerBound ? ['--lower-bound', JSON.stringify(lowerBound)] : []),
        node,
        query,
      ]
      const process = stream('subscribe', args)
      process.stdout &&
        (await handleStreamResponse(QueryResponse, onData, process.stdout, () => process.cancel()))
    },
    subscribeMonotonic: async (request, onData) => {
      const { lowerBound, query, session } = request
      const args = [
        ...['--session', session],
        ...(lowerBound ? ['--lower-bound', JSON.stringify(lowerBound)] : []),
        node,
        query,
      ]
      const process = stream('subscribe_monotonic', args)
      process.stdout &&
        (await handleStreamResponse(SubscribeMonotonicResponse, onData, process.stdout, () =>
          process.cancel(),
        ))
    },
  }
}

// eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
export const mkExec = (binary: string, addr: string, identityPath: string): Exec => ({
  version: async () => {
    const response = await execa(path.resolve(binary), ['--version'])
    return response.stdout
  },
  apps: {
    sign: (devCertFilePath, appManifestFilePath) =>
      exec(binary, ['apps', 'sign', devCertFilePath, appManifestFilePath]).then((x) =>
        rightOrThrow(ResponseAppsSign.decode(x), x),
      ),
  },
  users: {
    keyGen: async (file: string): Promise<Response_Users_Keygen> => {
      const response = await exec(binary, ['users', 'keygen', '--output', file])
      return rightOrThrow(Response_Users_Keygen.decode(response), response)
    },
  },
  swarms: {
    keyGen: async (file): Promise<Response_Swarms_Keygen> => {
      const fileArgs = file ? ['-o', file] : []
      const response = await exec(binary, ['swarms', 'keygen', ...fileArgs])
      return rightOrThrow(Response_Swarms_Keygen.decode(response), response)
    },
  },
  nodes: {
    ls: async (): Promise<Response_Nodes_Ls> => {
      const response = await exec(binary, [`nodes`, `ls`, ...addr.split(' '), '-i', identityPath])
      return rightOrThrow(Response_Nodes_Ls.decode(response), response)
    },
    inspect: async (): Promise<Response_Nodes_Inspect> => {
      const json = await exec(binary, ['nodes', 'inspect', addr, '-i', identityPath])
      return rightOrThrow(Response_Nodes_Inspect.decode(json), json)
    },
  },
  settings: {
    get: async (scope: string, noDefaults?: boolean): Promise<Response_Settings_Get> => {
      const response = await exec(
        binary,
        ['settings', 'get', scope, addr, '-i', identityPath].concat(
          noDefaults ? ['--no-defaults'] : [],
        ),
      )
      return rightOrThrow(Response_Settings_Get.decode(response), response)
    },
    set: async (scope: string, settingsInput: SettingsInput): Promise<Response_Settings_Set> => {
      const input = SettingsInput.match({
        File: (input) => `@${input.path}`,
        Value: (input) => JSON.stringify(input.value),
      })(settingsInput)
      const response = await exec(binary, [
        `settings`,
        `set`,
        scope,
        input,
        addr,
        '-i',
        identityPath,
      ])
      return rightOrThrow(Response_Settings_Set.decode(response), response)
    },
    unset: async (scope: string): Promise<Response_Settings_Unset> => {
      const response = await exec(binary, [`settings`, `unset`, scope, addr, '-i', identityPath])
      return rightOrThrow(Response_Settings_Unset.decode(response), response)
    },
    schema: async () => {
      const response = await exec(binary, [`settings`, `schema`, addr, '-i', identityPath])
      return rightOrThrow(Response_Settings_Schema.decode(response), response)
    },
  },
  internal: {
    shutdown: async (): Promise<void> => {
      const response = await exec(binary, [`internal`, `shutdown`, addr, `-i`, identityPath], {
        env: { HERE_BE_DRAGONS: 'zøg' },
      })
      if (response.code !== 'OK' && response.code !== 'ERR_NODE_UNREACHABLE') {
        throw new Error(`shutdown failed: ${JSON.stringify(response)}`)
      }
    },
  },
})
