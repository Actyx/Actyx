import { Loggers } from '../util/logging'

export const log = {
  test: Loggers.of('test'),
  ipfs: Loggers.of('ipfs'),
}
