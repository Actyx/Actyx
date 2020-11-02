interface Response {
  code: string
}

export const hasReponseCodeStatus = <T extends Response>(status: string) => (
  response: T,
): boolean => response.code === status

export const isCodeOk = hasReponseCodeStatus('OK')
export const isCodeInvalidInput = hasReponseCodeStatus('ERR_INVALID_INPUT')
export const isCodeNodeUnreachable = hasReponseCodeStatus('ERR_NODE_UNREACHABLE')
