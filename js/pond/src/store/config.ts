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
