import {
  Response_Nodes_Ls,
  Response_Settings_Set,
  Response_Settings_Unset,
  Response_Apps_Package,
  Response_Apps_Deploy,
  Response_Apps_Undeploy,
  Response_Apps_Start,
  Response_Apps_Stop,
  Response_Apps_Ls,
  Response_Logs_Tail_Entry,
} from './types'
import { Either, isLeft } from 'fp-ts/lib/Either'
import { Errors } from 'io-ts'
import { PathReporter } from 'io-ts/lib/PathReporter'
import * as execa from 'execa'
import { StringDecoder } from 'string_decoder'
import { Transform } from 'stream'

const rightOrThrow = <A>(e: Either<Errors, A>): A => {
  if (isLeft(e)) {
    throw new Error(PathReporter.report(e).join(', '))
  }
  return e.right
}

const exec = async (binary: string, args: string[]) => {
  try {
    return JSON.parse((await execa(binary, [`-j`].concat(args))).stdout)
  } catch (error) {
    return JSON.parse(error.stdout)
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

// eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
export const mkExec = (binary: string, addr: string) => ({
  Nodes: {
    Ls: async (): Promise<Response_Nodes_Ls> => {
      const response = await exec(binary, [`nodes`, `ls`, `--local`, addr])
      return rightOrThrow(Response_Nodes_Ls.decode(response))
    },
  },
  Settings: {
    Set: async (scope: string, settingsInput: SettingsInput): Promise<Response_Settings_Set> => {
      const input = SettingsInput.match({
        File: (input) => `@${input.path}`,
        Value: (input) => JSON.stringify(input.value),
      })(settingsInput)
      const response = await exec(binary, [`settings`, `set`, scope, `--local`, input, addr])
      return rightOrThrow(Response_Settings_Set.decode(response))
    },
    Unset: async (scope: string): Promise<Response_Settings_Unset> => {
      const response = await exec(binary, [`settings`, `unset`, scope, `--local`, addr])
      return rightOrThrow(Response_Settings_Unset.decode(response))
    },
  },
  Apps: {
    Package: async (path: string): Promise<Response_Apps_Package> => {
      const response = await exec(binary, [`apps`, `package`, path])
      return rightOrThrow(Response_Apps_Package.decode(response))
    },
    Deploy: async (packagePath: string, force?: boolean): Promise<Response_Apps_Deploy> => {
      const response = await exec(
        binary,
        [`apps`, `deploy`, packagePath, `--local`, addr].concat(force ? ['--force'] : []),
      )
      return rightOrThrow(Response_Apps_Deploy.decode(response))
    },
    Undeploy: async (appId: string): Promise<Response_Apps_Undeploy> => {
      const response = await exec(binary, [`apps`, `undeploy`, appId, `--local`, addr])
      return rightOrThrow(Response_Apps_Undeploy.decode(response))
    },
    Start: async (appId: string): Promise<Response_Apps_Start> => {
      const response = await exec(binary, [`apps`, `start`, appId, `--local`, addr])
      return rightOrThrow(Response_Apps_Start.decode(response))
    },
    Stop: async (appId: string): Promise<Response_Apps_Stop> => {
      const response = await exec(binary, [`apps`, `stop`, appId, `--local`, addr])
      return rightOrThrow(Response_Apps_Stop.decode(response))
    },
    Ls: async (): Promise<Response_Apps_Ls> => {
      const response = await exec(binary, [`apps`, `ls`, `--local`, addr])
      return rightOrThrow(Response_Apps_Ls.decode(response))
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
          } else if (entry.right.code !== 'OK') {
            onError(`${entry.right.code}: ${entry.right.message}`)
          } else {
            onEntry(entry.right)
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
        return () => {}
      }
    },
  },
})
