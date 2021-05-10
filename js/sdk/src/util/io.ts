/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2021 Actyx AG
 */
import * as t from 'io-ts'

/* eslint-disable @typescript-eslint/no-explicit-any */

const validationErrorsMsgs = (input: any, decoder: string, errors: t.Errors) => {
  const validationErrors = errors.map(
    error => `[${error.context.map(({ key }) => key).join('.')}] = ${JSON.stringify(error.value)}`,
  )
  return `Validation of [${JSON.stringify(input)}] to ${decoder} failed:\n${validationErrors.join(
    '.',
  )}`
}

const isTestEnv =
  typeof process !== 'undefined' && process.env && process.env.NODE_ENV !== 'production'

// Just as cast in production
export const validateOrThrow: <T>(decoder: t.Decoder<any, T>) => (value: any) => T = decoder =>
  isTestEnv
    ? (value: any) =>
        decoder.decode(value).fold(errors => {
          throw new Error(validationErrorsMsgs(value, decoder.name, errors))
        }, x => x)
    : <T>(value: any) => value as T
