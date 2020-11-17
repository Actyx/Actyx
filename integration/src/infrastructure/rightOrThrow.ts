import { Either, isLeft } from 'fp-ts/lib/Either'
import { Errors } from 'io-ts'

export const rightOrThrow = <A>(e: Either<Errors, A>, obj: unknown): A => {
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
