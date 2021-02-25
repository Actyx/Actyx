import {
  Response_Nodes_Ls,
  Response_Settings_Get,
  Response_Settings_Set,
  Response_Settings_Unset,
  Response_Logs_Tail_Entry,
  Response_Internal_Swarm_State,
  Response_Settings_Scopes,
  Response_Settings_Schema,
  Response_Swarms_Keygen,
  Response_Users_Keygen,
} from './types'
import { isLeft } from 'fp-ts/lib/Either'
import { PathReporter } from 'io-ts/lib/PathReporter'
import execa from 'execa'
import { StringDecoder } from 'string_decoder'
import { Transform } from 'stream'
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
  users: {
    keyGen: (file: string) => Promise<Response_Users_Keygen>
  }
  swarms: {
    keyGen: (file?: string) => Promise<Response_Swarms_Keygen>
    state: () => Promise<Response_Internal_Swarm_State>
  }
  nodes: {
    ls: () => Promise<Response_Nodes_Ls>
  }
  settings: {
    scopes: () => Promise<Response_Settings_Scopes>
    get: (scope: string, noDefaults?: boolean) => Promise<Response_Settings_Get>
    set: (scope: string, input: SettingsInput) => Promise<Response_Settings_Set>
    unset: (scope: string) => Promise<Response_Settings_Unset>
    schema: (scope: string) => Promise<Response_Settings_Schema>
  }
  logs: {
    tailFollow: (
      onEntry: (entry: Response_Logs_Tail_Entry) => void,
      onError: (error: string) => void,
    ) => () => void
  }
}

// eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
export const mkExec = (binary: string, addr: string, identityPath: string): Exec => ({
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
    state: async (): Promise<Response_Internal_Swarm_State> => {
      const json = await exec(binary, ['_internal', 'swarm', '--local', addr, '-i', identityPath])
      return rightOrThrow(Response_Internal_Swarm_State.decode(json), json)
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
  logs: {
    tailFollow: (
      onEntry: (entry: Response_Logs_Tail_Entry) => void,
      onError: (error: string) => void,
    ): (() => void) => {
      try {
        //console.log(`starting ax process`)
        const process = execa(
          `ax`,
          [`-j`, `logs`, `tail`, `-f`, `--local`, addr, '-i', identityPath],
          {
            buffer: false,
          },
        )
        if (process.stdout === null) {
          onError(`stdout is null`)
          // eslint-disable-next-line @typescript-eslint/no-empty-function
          return () => {}
        }
        //console.log(`got non-null stdout`)

        const utf8Decoder = new StringDecoder('utf8')

        let last = ''

        //console.log(`creating decoder`)
        const entryDecoder = new Transform({
          readableObjectMode: true,
          transform(chunk, _, cb) {
            let lines: string[] = []
            try {
              last += utf8Decoder.write(chunk)
              const list = last.split(/\r?\n/)
              const p = list.pop()
              last = p === undefined ? '' : p
              lines = list.filter((x) => x.length > 0)
            } catch (err) {
              cb(err)
              return
            }

            if (lines.length > 0) {
              lines.forEach((l) => this.push(l))
              cb(null)
            } else {
              cb()
            }
          },
        })

        // This is set to non-null if the request has an error. Otherwise
        // if returns none (this happens only when the connection is
        // manually aborted using the returned function).
        let error: string | null = null
        //console.log(`piping stdout to decoder`)
        process.stdout.pipe(entryDecoder).on('data', (str) => {
          //console.log(`got data: '${str}'`)
          const val = JSON.parse(str)
          const entry = Response_Logs_Tail_Entry.decode(val)
          if (isLeft(entry)) {
            //console.log(`error decoding log entry: ${PathReporter.report(entry).join(', ')}`)
            error = `error decoding log entry response: ${PathReporter.report(entry).join(', ')}`
            process.kill()
          } else if (entry.value.code !== 'OK') {
            onError(`${entry.value.code}: ${entry.value.message}`)
          } else {
            onEntry(entry.value)
          }
        })

        const killProcess = () => {
          //console.log(`killing process`)
          process.kill()
          // TODO
        }

        process.stdout.on('error', (err: string) => {
          //console.log(`got error: ${err}`)
          error = err
        })
        process.stdout.on('close', (err: string) => {
          //console.log(`stream closing (err: ${err})`)
          process.kill()
          if (error === null) {
            // Nothing happens here
          } else if (err !== '' && err !== undefined && err !== null) {
            onError(err)
          } else {
            onError(error as string)
          }
        })

        return () => {
          killProcess()
        }
      } catch (error) {
        //console.log(`caught error (err: ${error})`)
        onError(error.toString())
        // eslint-disable-next-line @typescript-eslint/no-empty-function
        return () => {}
      }
    },
  },
})
