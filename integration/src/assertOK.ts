export const assertOK = <T>(obj: T): T & { code: 'OK' } => {
  expect(obj).toMatchCodeOk()
  return obj as T & { code: 'OK' }
}
