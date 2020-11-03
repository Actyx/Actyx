import {
  Response_Nodes_Ls,
  Response_Settings_Get,
  Response_Settings_Set,
  Response_Settings_Unset,
  Response_Apps_Package,
  Response_Apps_Deploy,
  Response_Apps_Undeploy,
  Response_Apps_Start,
  Response_Apps_Stop,
  Response_Apps_Ls,
  Response_Logs_Tail_Entry,
  Response_Internal_Swarm_State,
  Response_Settings_Scopes,
  Response_Settings_Schema,
  Reponse_Swarms_Keygen,
  Reponse_Apps_Validate,
} from './types'
import { Either, isLeft } from 'fp-ts/lib/Either'
import { Errors } from 'io-ts'
import { PathReporter } from 'io-ts/lib/PathReporter'
import execa from 'execa'
import { StringDecoder } from 'string_decoder'
import { Transform } from 'stream'
import fetch from 'node-fetch'

const rightOrThrow = <A>(e: Either<Errors, A>, obj: unknown): A => {
  if (isLeft(e)) {
    throw new Error(
      e.value
        .map((err) => {
          const path = err.context.map(({ key }) => key).join('.')
          return `invalid ${err.value} at ${path}: ${err.message}`
        })
        .join(', ') +
        ' while parsing ' +
        JSON.stringify(obj, null, 2),
    )
  }
  return e.value
}

const exec = async (binary: string, args: string[], cwd?: string) => {
  try {
    const option: execa.Options | undefined = cwd ? { cwd } : undefined
    const response = (await execa((cwd ? '../../../' : '') + binary, [`-j`].concat(args), option))
      .stdout
    return JSON.parse(response)
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
  Swarms: {
    KeyGen: (file?: string) => Promise<Reponse_Swarms_Keygen>
    State: () => Promise<Response_Internal_Swarm_State>
  }
  Nodes: {
    Ls: () => Promise<Response_Nodes_Ls>
  }
  Settings: {
    Scopes: () => Promise<Response_Settings_Scopes>
    Get: (scope: string) => Promise<Response_Settings_Get>
    Set: (scope: string, input: SettingsInput) => Promise<Response_Settings_Set>
    Unset: (scope: string) => Promise<Response_Settings_Unset>
    Schema: (scope: string) => Promise<Response_Settings_Schema>
  }
  Apps: {
    Package: (path: string) => Promise<Response_Apps_Package>
    Deploy: (packagePath: string, force?: boolean) => Promise<Response_Apps_Deploy>
    Undeploy: (appId: string) => Promise<Response_Apps_Undeploy>
    Start: (appId: string) => Promise<Response_Apps_Start>
    Stop: (appId: string) => Promise<Response_Apps_Stop>
    Ls: () => Promise<Response_Apps_Ls>
    Validate: (filePath?: string) => Promise<Reponse_Apps_Validate>
    ValidateCwd: (cwd: string) => Promise<Reponse_Apps_Validate>
  }
  Logs: {
    TailFollow: (
      onEntry: (entry: Response_Logs_Tail_Entry) => void,
      onError: (error: string) => void,
    ) => () => void
  }
}

// eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
export const mkExec = (binary: string, addr: string): Exec => ({
  Swarms: {
    KeyGen: async (file): Promise<Reponse_Swarms_Keygen> => {
      const fileArgs = file ? ['-o', file] : []
      const response = await exec(binary, ['swarms', 'keygen', ...fileArgs])
      return rightOrThrow(Reponse_Swarms_Keygen.decode(response), response)
    },
    State: async (): Promise<Response_Internal_Swarm_State> => {
      const response = await fetch(`http://${addr}/_internal/swarm/state`)
      const json = await response.json()
      return rightOrThrow(Response_Internal_Swarm_State.decode(json), json)
    },
  },
  Nodes: {
    Ls: async (): Promise<Response_Nodes_Ls> => {
      const response = await exec(binary, [`nodes`, `ls`, `--local`, ...addr.split(' ')])
      return rightOrThrow(Response_Nodes_Ls.decode(response), response)
    },
  },
  Settings: {
    Scopes: async () => {
      const response = await exec(binary, ['settings', 'scopes', '--local', addr])
      return rightOrThrow(Response_Settings_Scopes.decode(response), response)
    },
    Get: async (scope: string): Promise<Response_Settings_Get> => {
      const response = await exec(binary, ['settings', 'get', scope, '--local', addr])
      return rightOrThrow(Response_Settings_Get.decode(response), response)
    },
    Set: async (scope: string, settingsInput: SettingsInput): Promise<Response_Settings_Set> => {
      const input = SettingsInput.match({
        File: (input) => `@${input.path}`,
        Value: (input) => JSON.stringify(input.value),
      })(settingsInput)
      const response = await exec(binary, [`settings`, `set`, scope, `--local`, input, addr])
      return rightOrThrow(Response_Settings_Set.decode(response), response)
    },
    Unset: async (scope: string): Promise<Response_Settings_Unset> => {
      const response = await exec(binary, [`settings`, `unset`, scope, `--local`, addr])
      return rightOrThrow(Response_Settings_Unset.decode(response), response)
    },
    Schema: async (scope: string) => {
      const response = await exec(binary, [`settings`, `schema`, `--local`, scope, addr])
      return rightOrThrow(Response_Settings_Schema.decode(response), response)
    },
  },
  Apps: {
    Package: async (path: string): Promise<Response_Apps_Package> => {
      const response = await exec(binary, [`apps`, `package`, path])
      return rightOrThrow(Response_Apps_Package.decode(response), response)
    },
    Deploy: async (packagePath: string, force?: boolean): Promise<Response_Apps_Deploy> => {
      const response = await exec(
        binary,
        [`apps`, `deploy`, packagePath, `--local`, addr].concat(force ? ['--force'] : []),
      )
      return rightOrThrow(Response_Apps_Deploy.decode(response), response)
    },
    Undeploy: async (appId: string): Promise<Response_Apps_Undeploy> => {
      const response = await exec(binary, [`apps`, `undeploy`, appId, `--local`, addr])
      return rightOrThrow(Response_Apps_Undeploy.decode(response), response)
    },
    Start: async (appId: string): Promise<Response_Apps_Start> => {
      const response = await exec(binary, [`apps`, `start`, appId, `--local`, addr])
      return rightOrThrow(Response_Apps_Start.decode(response), response)
    },
    Stop: async (appId: string): Promise<Response_Apps_Stop> => {
      const response = await exec(binary, [`apps`, `stop`, appId, `--local`, addr])
      return rightOrThrow(Response_Apps_Stop.decode(response), response)
    },
    Ls: async (): Promise<Response_Apps_Ls> => {
      const response = await exec(binary, [`apps`, `ls`, `--local`, addr])
      return rightOrThrow(Response_Apps_Ls.decode(response), response)
    },
    Validate: async (filePath?: string): Promise<Reponse_Apps_Validate> => {
      const response = await exec(binary, [`apps`, `validate`, filePath ?? ''])
      return rightOrThrow(Reponse_Apps_Validate.decode(response), response)
    },
    ValidateCwd: async (cwd: string): Promise<Reponse_Apps_Validate> => {
      const response = await exec(binary, [`apps`, `validate`], cwd)
      return rightOrThrow(Reponse_Apps_Validate.decode(response), response)
    },
  },
  Logs: {
    TailFollow: (
      onEntry: (entry: Response_Logs_Tail_Entry) => void,
      onError: (error: string) => void,
    ): (() => void) => {
      try {
        //console.log(`starting ax process`)
        const process = execa(`ax`, [`-j`, `logs`, `tail`, `-f`, `--local`, addr], {
          buffer: false,
        })
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
