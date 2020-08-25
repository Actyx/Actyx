/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */

/** Actyx store communication configuration. @public */
export type Config = Readonly<{
  monitoringMeta?: object
  /**
   * Interval at which to send metadata messages via pubsub; the first metadata are
   * sent immediately.
   */
  metaMs: number
  /**
   * Run stats frequency
   */
  runStatsPeriodMs: number
}>

export const defaultConfig = (): Config => ({
  runStatsPeriodMs: 60000,
  metaMs: 3600000,
})
