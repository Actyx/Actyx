/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
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
