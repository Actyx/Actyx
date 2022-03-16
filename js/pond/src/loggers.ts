/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2020 Actyx AG
 */
import { Loggers } from '@actyx/sdk/lib/util/logging'
/**
 * Scope for things related to the pond
 */
const pond = Loggers.of('pond')
const chaos = Loggers.of('chaos')
const http = Loggers.of('http')
const ws = Loggers.of('ws')
const submono = Loggers.of('subscribe_monotonic')
export const log = { pond, chaos, http, ws, submono }
export default log
