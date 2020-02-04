import { Loggers } from '../util/logging'

export const log = {
  /**
   * Scope for things related to the ws connection to the backend (RDS and others)
   */
  ws: Loggers.of('ws'),
  monitoring: Loggers.of('monitoring'),
  stats: Loggers.of('stats'),
}

export default log
