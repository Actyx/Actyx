type Logger = {
  log: (where: 'stdout' | 'stderr', s: Buffer | string) => string[] | undefined
  flush: () => void
}

export const netString = (x: Buffer | string): string =>
  Buffer.isBuffer(x) ? x.toString('utf-8') : x

export const mkProcessLogger = (
  logger: (s: string) => void,
  nodeName: string,
  triggers: string[],
): Logger => {
  const lines = { stdout: '', stderr: '' }
  const log = (where: keyof typeof lines, s: Buffer | string) => {
    const l = (lines[where] + netString(s)).split('\n')
    lines[where] = l.pop() || ''
    const matchedLines = []
    for (const line of l) {
      logger(`node ${nodeName} Actyx ${where}: ${line}`)
      if (triggers.some((s) => line.indexOf(s) >= 0)) {
        matchedLines.push(line)
      }
    }
    return matchedLines.length > 0 ? matchedLines : undefined
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
