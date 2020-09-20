import * as io from 'io-ts'

const _OK = io.type({
  code: io.literal('OK'),
})

const ERR_APP_ENABLED = io.type({
  code: io.literal('ERR_APP_ENABLED'),
  message: io.string,
})
const ERR_INVALID_INPUT = io.type({
  code: io.literal('ERR_INVALID_INPUT'),
  message: io.string,
})
const ERR_SETTINGS_INVALID = io.type({
  code: io.literal('ERR_SETTINGS_INVALID'),
  message: io.string,
})
const ERR_SETTINGS_UNKNOWN_SCOPE = io.type({
  code: io.literal('ERR_SETTINGS_UNKNOWN_SCOPE'),
  message: io.string,
})
const ERR_APP_INVALID_PACKAGE = io.type({
  code: io.literal('ERR_APP_INVALID_PACKAGE'),
  message: io.string,
})
const ERR_APP_ALREADY_DEPLOYED = io.type({
  code: io.literal('ERR_APP_ALREADY_DEPLOYED'),
  message: io.string,
})
const ERR_NODE_UNREACHABLE = io.type({
  code: io.literal('ERR_NODE_UNREACHABLE'),
  message: io.string,
})

const ERR_NODE_MISCONFIGURED = io.type({
  code: io.literal('ERR_NODE_MISCONFIGURED'),
  message: io.string,
})

export const Response_Nodes_Ls = io.intersection([
  _OK,
  io.type({
    result: io.array(
      io.union([
        io.type({
          connection: io.literal('reachable'),
          nodeId: io.string,
          displayName: io.union([io.null, io.string]),
          state: io.literal('running'),
          settingsValid: io.boolean,
          licensed: io.boolean,
          appsDeployed: io.Integer,
          appsRunning: io.Integer,
          startedIso: io.string,
          startedUnix: io.Integer,
          version: io.string,
        }),
        io.type({
          connection: io.literal('hostUnreachable'),
          host: io.string,
        }),
        io.type({
          connection: io.literal('actyxosUnreachable'),
          host: io.string,
        }),
      ]),
    ),
  }),
])

export type Response_Nodes_Ls = io.TypeOf<typeof Response_Nodes_Ls>

export const Response_Settings_Set = io.union([
  ERR_APP_ENABLED,
  ERR_INVALID_INPUT,
  ERR_SETTINGS_INVALID,
  ERR_NODE_UNREACHABLE,
  io.intersection([
    _OK,
    io.type({
      result: io.type({
        scope: io.string,
        settings: io.unknown,
      }),
    }),
  ]),
])

export type Response_Settings_Set = io.TypeOf<typeof Response_Settings_Set>

export const Response_Settings_Unset = io.union([
  ERR_APP_ENABLED,
  ERR_SETTINGS_UNKNOWN_SCOPE,
  ERR_NODE_UNREACHABLE,
  io.intersection([
    _OK,
    io.type({
      result: io.type({}),
    }),
  ]),
])

export type Response_Settings_Unset = io.TypeOf<typeof Response_Settings_Unset>

export const Response_Apps_Package = io.union([
  ERR_INVALID_INPUT,
  io.intersection([
    _OK,
    io.type({
      result: io.array(
        io.type({
          appId: io.string,
          appVersion: io.string,
          packagePath: io.string,
        }),
      ),
    }),
  ]),
])

export type Response_Apps_Package = io.TypeOf<typeof Response_Apps_Package>

export const Response_Apps_Deploy = io.union([
  ERR_INVALID_INPUT,
  ERR_APP_INVALID_PACKAGE,
  ERR_APP_ALREADY_DEPLOYED,
  ERR_NODE_UNREACHABLE,
  io.intersection([
    _OK,
    io.type({
      result: io.type({
        redeployed: io.boolean,
      }),
    }),
  ]),
])

export type Response_Apps_Deploy = io.TypeOf<typeof Response_Apps_Deploy>

export const Response_Apps_Undeploy = io.union([
  ERR_INVALID_INPUT,
  ERR_APP_ENABLED,
  ERR_NODE_UNREACHABLE,
  io.intersection([
    _OK,
    io.type({
      result: io.type({
        appId: io.string,
        host: io.string,
      }),
    }),
  ]),
])

export type Response_Apps_Undeploy = io.TypeOf<typeof Response_Apps_Undeploy>

export const Response_Apps_Start = io.union([
  ERR_INVALID_INPUT,
  ERR_NODE_UNREACHABLE,
  ERR_NODE_MISCONFIGURED,
  io.intersection([
    _OK,
    io.type({
      result: io.type({
        appId: io.string,
        host: io.string,
        alreadyStarted: io.boolean,
      }),
    }),
  ]),
])

export type Response_Apps_Start = io.TypeOf<typeof Response_Apps_Start>

export const Response_Apps_Stop = io.union([
  ERR_INVALID_INPUT,
  ERR_NODE_UNREACHABLE,
  io.intersection([
    _OK,
    io.type({
      result: io.type({
        appId: io.string,
        host: io.string,
        alreadyStopped: io.boolean,
      }),
    }),
  ]),
])

export type Response_Apps_Stop = io.TypeOf<typeof Response_Apps_Stop>

export const Response_Apps_Ls = io.union([
  ERR_NODE_UNREACHABLE,
  io.intersection([
    _OK,
    io.type({
      result: io.array(
        io.type({
          nodeId: io.string,
          appId: io.string,
          version: io.string,
          running: io.boolean,
          startedIso: io.union([io.null, io.string]),
          startedUnix: io.union([io.null, io.number]),
          licensed: io.boolean,
          settingsValid: io.boolean,
          enabled: io.boolean,
        }),
      ),
    }),
  ]),
])

export type Response_Apps_Ls = io.TypeOf<typeof Response_Apps_Ls>

export const Response_Logs_Tail_Entry = io.union([
  ERR_NODE_UNREACHABLE,
  io.intersection([
    _OK,
    io.type({
      result: io.type({
        sequenceNumber: io.number,
        logTimestamp: io.string,
        nodeId: io.string,
        nodeName: io.string,
        severity: io.union([
          io.literal('DEBUG'),
          io.literal('WARN'),
          io.literal('INFO'),
          io.literal('ERROR'),
        ]),
        message: io.string,
        logName: io.string,
        additionalData: io.unknown,
        labels: io.union([io.null, io.record(io.string, io.string)]),
        producerName: io.string,
        producerVersion: io.string,
      }),
    }),
  ]),
])

export type Response_Logs_Tail_Entry = io.TypeOf<typeof Response_Logs_Tail_Entry>
