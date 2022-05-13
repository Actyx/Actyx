/* eslint-disable @typescript-eslint/no-explicit-any */
export const massageError = (err: any): any => {
  if (Array.isArray(err)) {
    return err.map((elem) => massageError(elem))
  }
  if (typeof err !== 'object' || err === null) {
    return err
  }
  if (err.constructor.name === 'WebSocket') {
    return 'WebSocket'
  }
  Object.assign(
    err,
    Object.fromEntries(
      Object.getOwnPropertySymbols(err).map((n) => [n.toString(), massageError(err[n])]),
    ),
  )
  const e = new Error(stringifyError(err))
  delete e.stack
  return e
}

export const stringifyError = (err: any) => {
  try {
    return JSON.stringify(err)
  } catch (_e) {
    return `${err}`
  }
}
