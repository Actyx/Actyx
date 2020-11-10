interface Response {
  code: string
}

export const hasReponseCodeStatus = <T extends Response>(status: string) => (
  response: T,
): boolean => response.code === status

// TODO: remove
export const isCodeOk = hasReponseCodeStatus('OK')
