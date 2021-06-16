import {
  ResponseAppsSign,
  Response_Nodes_Ls,
  Response_Nodes_Inspect,
  Response_Settings_Get,
  Response_Settings_Set,
  Response_Settings_Unset,
  Response_Settings_Scopes,
  Response_Settings_Schema,
  Response_Swarms_Keygen,
  Response_Users_Keygen,
} from './types'
import execa from 'execa'
import * as path from 'path'
import { rightOrThrow } from '../infrastructure/rightOrThrow'

const exec = async (binaryPath: string, args: string[], cwd?: string) => {
  try {
    const option: execa.Options | undefined = cwd ? { cwd } : undefined
    const binaryPathResolved = path.resolve(binaryPath)
    const response = await execa(binaryPathResolved, [`-j`].concat(args), option)
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
    scopes: () => Promise<Response_Settings_Scopes>
    get: (scope: string, noDefaults?: boolean) => Promise<Response_Settings_Get>
    set: (scope: string, input: SettingsInput) => Promise<Response_Settings_Set>
    unset: (scope: string) => Promise<Response_Settings_Unset>
    schema: (scope: string) => Promise<Response_Settings_Schema>
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
      const response = await exec(binary, [
        `nodes`,
        `ls`,
        `--local`,
        ...addr.split(' '),
        '-i',
        identityPath,
      ])
      return rightOrThrow(Response_Nodes_Ls.decode(response), response)
    },
    inspect: async (): Promise<Response_Nodes_Inspect> => {
      const json = await exec(binary, ['nodes', 'inspect', '--local', addr, '-i', identityPath])
      return rightOrThrow(Response_Nodes_Inspect.decode(json), json)
    },
  },
  settings: {
    scopes: async () => {
      const response = await exec(binary, [
        'settings',
        'scopes',
        '--local',
        addr,
        '-i',
        identityPath,
      ])
      return rightOrThrow(Response_Settings_Scopes.decode(response), response)
    },
    get: async (scope: string, noDefaults?: boolean): Promise<Response_Settings_Get> => {
      const response = await exec(
        binary,
        ['settings', 'get', scope, '--local', addr, '-i', identityPath].concat(
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
        `--local`,
        input,
        addr,
        '-i',
        identityPath,
      ])
      return rightOrThrow(Response_Settings_Set.decode(response), response)
    },
    unset: async (scope: string): Promise<Response_Settings_Unset> => {
      const response = await exec(binary, [
        `settings`,
        `unset`,
        scope,
        `--local`,
        addr,
        '-i',
        identityPath,
      ])
      return rightOrThrow(Response_Settings_Unset.decode(response), response)
    },
    schema: async (scope: string) => {
      const response = await exec(binary, [
        `settings`,
        `schema`,
        `--local`,
        scope,
        addr,
        '-i',
        identityPath,
      ])
      return rightOrThrow(Response_Settings_Schema.decode(response), response)
    },
  },
})
