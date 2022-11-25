import fetch from 'cross-fetch'
import { NodeInfo as NodeInfoIo } from './internal_common'
import * as SV from 'semver'
import * as E from 'fp-ts/Either'
import { ActyxOpts } from '.'
import log from './internal_common/log'
import { pipe } from 'fp-ts/function'
import { getApiLocation, mkHeaders } from './v2/utils'

let lastInfoTime = 0
let lastInfo: NodeInfo | null = null

export const getInfo = (config: ActyxOpts) => {
  const uri = `http://${getApiLocation(config.actyxHost, config.actyxPort)}/node/info`
  return async (token: string, maxAgeMillis: number): Promise<NodeInfo> => {
    if (Date.now() - maxAgeMillis <= lastInfoTime) {
      return lastInfo || invalidNodeInfo
    }
    const resp = await fetch(uri, { method: 'get', headers: mkHeaders(token) })
    if (resp.status === 404) {
      log.actyx.warn(
        'The targeted node seems not to support the `/api/v2/node/info` endpoint. Consider updating to the latest version.',
      )
      return invalidNodeInfo
    }
    const json = await resp.json()
    return pipe(
      NodeInfoIo.decode(json),
      E.map((io) => new NodeInfo(io)),
      E.fold(
        (errors) => {
          log.actyx.error('errors while parsing NodeInfo response:', ...errors)
          return invalidNodeInfo
        },
        (x) => {
          lastInfoTime = Date.now()
          lastInfo = x
          return x
        },
      ),
    )
  }
}

/**
 * Accessor to information on the Actyx node this SDK is connected to.
 * @public
 */
export class NodeInfo {
  private semver: SV.SemVer

  constructor(private io: NodeInfoIo) {
    this.semver = SV.coerce(this.io.version) || new SV.SemVer('0.0.0')
  }

  /**
   *
   * @returns the full version string including git hash and CPU architecture
   */
  longVersion(): string {
    return this.io.version
  }

  /**
   *
   * @returns the semantic version part of the full version string, e.g. `2.6.1`
   */
  semVer(): string {
    return this.semver.format()
  }

  /**
   *
   * @param version - The version to compare with, in semantic version format `<major>.<minor>.<patch>`
   * @returns `true` if the reported Actyx version is greater than or equal to the supplied version
   */
  isAtLeastVersion(version: string): boolean {
    return SV.gte(this.semver, version)
  }

  /**
   *
   * @returns the uptime of the Actyx node in milliseconds
   */
  uptimeMillis() {
    return this.io.uptime.secs * 1000 + Math.floor(this.io.uptime.nanos / 1_000_000)
  }

  /**
   *
   * @returns the number of other nodes the Actyx node is currently connected to
   */
  connectedNodes(): number {
    return this.io.connectedNodes
  }
}

export const invalidNodeInfo = new NodeInfo({
  connectedNodes: -1,
  uptime: { secs: 0, nanos: 0 },
  version: '1.0.0-unknown',
})
