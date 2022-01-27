import stream from 'stream'

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
  print?: stream.Writable,
  prefix?: string,
): Logger => {
  const lines = { stdout: '', stderr: '' }
  const log = (where: keyof typeof lines, s: Buffer | string) => {
    const l = (lines[where] + netString(s)).split('\n')
    lines[where] = l.pop() || ''
    const matchedLines = []
    for (const lin of l) {
      if (print) {
        print.write(`${new Date().toISOString()} ${prefix || nodeName} ${lin}\n`)
      }
      // eslint-disable-next-line no-control-regex
      const line = lin.replace(/\u001b\[[^a-z]*[a-z]/g, '')
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
