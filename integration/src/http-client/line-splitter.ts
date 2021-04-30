import { Transform } from 'stream'
import { StringDecoder } from 'string_decoder'

/**
 * Copied over from Cosmos/js/os-sdk/src/client/request.ts
 */
export const mkLinesSplitter = function (): Transform {
  const utf8Decoder = new StringDecoder('utf8')
  let last = ''
  return new Transform({
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
}
