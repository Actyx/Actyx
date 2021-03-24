import * as io from 'io-ts'

const _OK = io.type({
  code: io.literal('OK'),
})

const ERR_INVALID_INPUT = io.type({
  code: io.literal('ERR_INVALID_INPUT'),
  message: io.string,
})
const ERR_INTERNAL_ERROR = io.type({
  code: io.literal('ERR_INTERNAL_ERROR'),
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
const ERR_SETTINGS_NOT_FOUND_AT_SCOPE = io.type({
  code: io.literal('ERR_SETTINGS_NOT_FOUND_AT_SCOPE'),
  message: io.string,
})
const ERR_NODE_UNREACHABLE = io.type({
  code: io.literal('ERR_NODE_UNREACHABLE'),
  message: io.string,
})
const ERR_FILE_EXISTS = io.type({
  code: io.literal('ERR_FILE_EXISTS'),
  message: io.string,
})

export const Response_Nodes_Ls = io.union([
  ERR_INTERNAL_ERROR,
  ERR_INVALID_INPUT,
  io.intersection([
    _OK,
    io.type({
      result: io.array(
        io.union([
          io.type({
            connection: io.literal('reachable'),
            host: io.string,
            nodeId: io.string,
            displayName: io.union([io.null, io.string]),
            startedIso: io.string,
            startedUnix: io.Integer,
            version: io.string,
          }),
          io.type({
            connection: io.literal('unreachable'),
            host: io.string,
          }),
          io.type({
            connection: io.literal('unauthorized'),
            host: io.string,
          }),
        ]),
      ),
    }),
  ]),
])

export type Response_Nodes_Ls = io.TypeOf<typeof Response_Nodes_Ls>

export const Response_Settings_Get = io.union([
  ERR_NODE_UNREACHABLE,
  ERR_INTERNAL_ERROR,
  ERR_SETTINGS_NOT_FOUND_AT_SCOPE,
  io.intersection([
    _OK,
    io.type({
      result: io.unknown,
    }),
  ]),
])

export type Response_Settings_Get = io.TypeOf<typeof Response_Settings_Get>

export const Response_Settings_Scopes = io.union([
  ERR_INTERNAL_ERROR,
  ERR_NODE_UNREACHABLE,
  io.intersection([
    _OK,
    io.type({
      result: io.array(io.string),
    }),
  ]),
])

export type Response_Settings_Scopes = io.TypeOf<typeof Response_Settings_Scopes>

export const Response_Settings_Schema = io.union([
  ERR_INTERNAL_ERROR,
  ERR_NODE_UNREACHABLE,
  io.intersection([
    _OK,
    io.type({
      result: io.UnknownRecord,
    }),
  ]),
])

export type Response_Settings_Schema = io.TypeOf<typeof Response_Settings_Schema>

export const Response_Settings_Set = io.union([
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

export const Response_Internal_Swarm_State = io.union([
  ERR_INVALID_INPUT,
  io.intersection([
    _OK,
    io.type({
      result: io.type({
        swarm: io.type({
          listen_addrs: io.array(io.string),
          peer_id: io.string,
          peers: io.record(
            io.string,
            io.type({
              addresses: io.record(
                io.string,
                io.type({
                  provenance: io.string,
                  state: io.union([
                    io.string,
                    io.type({ Connected: io.type({ since: io.number }) }),
                    io.type({ Disconnected: io.type({ since: io.number }) }),
                  ]),
                }),
              ),
              connection_state: io.string,
            }),
          ),
        }),
      }),
    }),
  ]),
])

export type Response_Internal_Swarm_State = io.TypeOf<typeof Response_Internal_Swarm_State>

export const Response_Swarms_Keygen = io.union([
  ERR_INVALID_INPUT,
  io.intersection([
    _OK,
    io.type({
      result: io.type({
        swarmKey: io.string,
        outputPath: io.union([io.string, io.null]),
      }),
    }),
  ]),
])

export type Response_Swarms_Keygen = io.TypeOf<typeof Response_Swarms_Keygen>

export const Response_Users_Keygen = io.union([
  ERR_FILE_EXISTS,
  io.intersection([
    _OK,
    io.type({
      result: io.type({
        privateKeyPath: io.string,
        publicKeyPath: io.string,
        publicKey: io.string,
      }),
    }),
  ]),
])

export type Response_Users_Keygen = io.TypeOf<typeof Response_Users_Keygen>
