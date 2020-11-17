type Logger = {
  log: (where: 'stdout' | 'stderr', s: Buffer | string) => boolean
  flush: () => void
}

const netString = (x: Buffer | string) => (Buffer.isBuffer(x) ? x.toString('utf-8') : x)

export const mkProcessLogger = (logger: (s: string) => void, nodeName: string): Logger => {
  const lines = { stdout: '', stderr: '' }
  const log = (where: keyof typeof lines, s: Buffer | string) => {
    const l = (lines[where] + netString(s)).split('\n')
    lines[where] = l.pop() || ''
    let startedSeen = false
    for (const line of l) {
      logger(`node ${nodeName} ActyxOS ${where}: ${line}`)
      if (line.indexOf('ActyxOS started') >= 0 || line.indexOf('ActyxOS ready') >= 0) {
        startedSeen = true
      }
    }
    return startedSeen
  }
  const flush = () => {
    if (lines.stdout !== '') {
      log('stdout', '\n')
    }
    if (lines.stderr !== '') {
      log('stderr', '\n')
    }
  }
  return { log, flush }
}
