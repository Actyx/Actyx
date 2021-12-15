/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import * as t from 'io-ts'
import { isLeft } from 'fp-ts/lib/Either'

/* eslint-disable @typescript-eslint/no-explicit-any */

const validationErrorsMsgs = (input: any, decoder: string, errors: t.Errors) => {
  const validationErrors = errors.map(
    (error) =>
      `[${error.context.map(({ key }) => key).join('.')}] = ${JSON.stringify(error.value)}`,
  )
  return `Validation of [${JSON.stringify(input)}] to ${decoder} failed:\n${validationErrors.join(
    '.',
  )}`
}

const isTestEnv =
  typeof process !== 'undefined' && process.env && process.env.NODE_ENV !== 'production'

export function validateOrThrow<T>(decoder: t.Decoder<any, T>) {
  if (isTestEnv) {
    return (value: any) => value as T
  }
  return (value: any) => {
    const validated = decoder.decode(value)
    if (isLeft(validated)) {
      throw new Error(validationErrorsMsgs(value, decoder.name, validated.left))
    }
    return validated.right
  }
}
