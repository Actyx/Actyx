declare module 'cids'
declare module 'format-util' {
  // eslint-disable-next-line
  declare var format: (msg: string, ...args: any[]) => string
  export = format
  export as namespace format
}
