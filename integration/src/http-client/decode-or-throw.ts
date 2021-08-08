import { fold, left, mapLeft } from 'fp-ts/lib/Either'
import { pipe } from 'fp-ts/lib/pipeable'
import { Decoder } from 'io-ts'
import { PathReporter } from 'io-ts/lib/PathReporter'

export const decodeOrThrow = <T>(decoder: Decoder<unknown, T>) => (data: unknown): T =>
  pipe(
    decoder.decode(data),
    mapLeft((e) => PathReporter.report(left(e))),
    fold(
      (x) => {
        throw new Error(x.join('\n'))
      },
      (x) => x,
    ),
  )
