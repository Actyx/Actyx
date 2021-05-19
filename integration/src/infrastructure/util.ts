// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const mkLog = (node: string) => (msg: string, ...rest: any[]): void =>
  console.log(`node ${node} ${msg}`, ...rest)
