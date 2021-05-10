/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2021 Actyx AG
 */
import { Loggers } from '../util/logging'

const http = Loggers.of('http')
const ws = Loggers.of('ws')
const submono = Loggers.of('subscribe_monotonic')
export const log = { http, ws, submono }
export default log
