import util from 'util'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const mkLog = (node: string) => (msg: string, ...rest: any[]): void => {
  process.stdout.write(util.format(`node ${node} ${msg}\n`, ...rest))
}

export const randIdentifier = (): string => Math.random().toString(36).substring(7)
