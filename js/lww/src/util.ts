export function toError(e: unknown): Error {
  return e instanceof Error ? e : new Error(String(e))
}
