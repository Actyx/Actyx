/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Loggers } from '../util/logging'

export const log = {
  ws: Loggers.of('ws'),
}
