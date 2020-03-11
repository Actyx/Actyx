/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Loggers } from './util/logging'
/**
 * Scope for things related to the pond
 */
const pond = Loggers.of('pond')
const chaos = Loggers.of('chaos')
const http = Loggers.of('http')
const ws = Loggers.of('ws')
export const log = { pond, chaos, http, ws }
export default log
