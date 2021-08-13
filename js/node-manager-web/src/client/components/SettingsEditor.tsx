import React, { useEffect, useReducer, useState } from 'react'
import clsx from 'clsx'
import { ReachableNode } from '../../common/types'
import deepEqual from 'deep-equal'
import { JsonEditor } from './JsonEditor'
import { Button } from './basics'
import { isValidMultiAddr, isValidMultiAddrWithPeerId } from '../../common/util'
import Ajv from 'ajv'
import draft6 from 'ajv/lib/refs/json-schema-draft-06.json'
import { useAppState } from '../app-state'
import { toUndefined } from 'fp-ts/lib/Option'

const eq = (a: Settings, b: Settings): boolean => deepEqual(a, b, { strict: false })

type ValidationError = string

const cachedSchemas: { [key: string]: object } = {}
const ajv = new Ajv()
ajv.addFormat('multiaddr-without-peer-id', isValidMultiAddr)
ajv.addFormat('multiaddr-with-peer-id', isValidMultiAddrWithPeerId)
ajv.addMetaSchema(draft6)

// This should be refactored at some point and hooked up with
// fatal error triggering
const validateAgainstSchema = (
  nodeAddr: string,
  data: object,
  schema: object,
): ValidationError[] => {
  if (!cachedSchemas[nodeAddr] || !eq(cachedSchemas[nodeAddr], schema)) {
    ajv.removeSchema(nodeAddr)
    if (!ajv.validateSchema(schema)) {
      throw `unable to validate schema ${JSON.stringify(ajv.errors)}`
    }

    try {
      ajv.addSchema(schema, nodeAddr)
      cachedSchemas[nodeAddr] = schema
    } catch (error) {
      try {
        ajv.removeSchema(nodeAddr)
      } catch (error) {
        console.error(error)
      }
    }
  }

  if (ajv.validate(nodeAddr, data)) {
    return []
  } else {
    if (ajv.errors) {
      return ajv.errors.map((e) => `${e.dataPath}: ${e.message}`)
    } else {
      throw "schema validator failed but didn't return errors"
    }
  }
}

type Settings = object | null

interface NotDiverged {
  key: 'NotDiverged'
  editor: Settings
}

interface DivergedFromInitial {
  key: 'DivergedFromInitial'
  editor: Settings
  initial: Settings
}

interface DivergedFromRemote {
  key: 'DivergedFromRemote'
  editor: Settings
  remote: Settings
}

interface DivergedFromBoth {
  key: 'DivergedFromBoth'
  editor: Settings
  remote: Settings
  initial: Settings
}

interface SavingToRemote {
  key: 'SavingToRemote'
  editor: Settings
}

interface SaveError {
  key: 'Error'
  editor: Settings
  reason: string
}

type State =
  | NotDiverged
  | DivergedFromInitial
  | DivergedFromRemote
  | DivergedFromBoth
  | SavingToRemote
  | SaveError

type Action =
  | { key: 'RemoteUpdated'; remote: Settings }
  | { key: 'EditorUpdated'; editor: Settings }
  | { key: 'Initial'; initial: Settings }
  | { key: 'SavingToRemote'; settings: Settings }
  | { key: 'SavingToRemoteFailed'; settings: Settings; reason: string }

const reducer = (current: State, action: Action): State => {
  switch (action.key) {
    case 'Initial': {
      return { key: 'NotDiverged', editor: action.initial }
    }
    case 'RemoteUpdated': {
      switch (current.key) {
        case 'NotDiverged': {
          if (!eq(current.editor, action.remote)) {
            return {
              key: 'DivergedFromRemote',
              editor: current.editor,
              remote: action.remote,
            }
          } else {
            return current
          }
        }
        case 'DivergedFromInitial': {
          if (!eq(current.editor, action.remote) && !eq(current.initial, action.remote)) {
            return {
              key: 'DivergedFromBoth',
              editor: current.editor,
              initial: current.initial,
              remote: action.remote,
            }
          } else {
            return current
          }
        }
        case 'DivergedFromRemote': {
          if (eq(current.remote, action.remote)) {
            return current
          } else if (eq(current.editor, action.remote)) {
            return { key: 'NotDiverged', editor: current.editor }
          } else {
            return {
              key: 'DivergedFromRemote',
              editor: current.editor,
              remote: action.remote,
            }
          }
        }
        case 'DivergedFromBoth': {
          if (eq(current.remote, action.remote)) {
            return current
          } else if (eq(current.editor, action.remote)) {
            return {
              key: 'DivergedFromInitial',
              editor: current.editor,
              initial: current.initial,
            }
          } else {
            return {
              key: 'DivergedFromBoth',
              editor: current.editor,
              initial: current.initial,
              remote: action.remote,
            }
          }
        }
        default: {
          return current
        }
      }
    }
    case 'EditorUpdated': {
      switch (current.key) {
        case 'NotDiverged': {
          if (!eq(current.editor, action.editor)) {
            return {
              key: 'DivergedFromInitial',
              initial: current.editor,
              editor: action.editor,
            }
          } else {
            return current
          }
        }
        case 'DivergedFromInitial': {
          if (eq(current.initial, action.editor)) {
            return { key: 'NotDiverged', editor: current.initial }
          } else {
            return {
              key: 'DivergedFromInitial',
              initial: current.initial,
              editor: action.editor,
            }
          }
        }
        case 'DivergedFromRemote': {
          if (eq(current.remote, action.editor)) {
            return { key: 'NotDiverged', editor: current.remote }
          } else {
            return {
              key: 'DivergedFromRemote',
              remote: current.remote,
              editor: action.editor,
            }
          }
        }
        case 'DivergedFromBoth': {
          if (eq(current.initial, action.editor) && eq(current.remote, action.editor)) {
            return { key: 'NotDiverged', editor: current.editor }
          } else if (eq(current.initial, action.editor)) {
            return {
              key: 'DivergedFromRemote',
              remote: current.remote,
              editor: action.editor,
            }
          } else if (eq(current.remote, action.editor)) {
            return {
              key: 'DivergedFromInitial',
              initial: current.initial,
              editor: action.editor,
            }
          } else {
            return {
              key: 'DivergedFromBoth',
              remote: current.remote,
              initial: current.initial,
              editor: action.editor,
            }
          }
        }
        default: {
          return current
        }
      }
    }
    case 'SavingToRemote': {
      return { key: 'SavingToRemote', editor: action.settings }
    }
    case 'SavingToRemoteFailed': {
      return { key: 'Error', editor: action.settings, reason: action.reason }
    }
  }
}

interface Props {
  node: ReachableNode
}

export const SettingsEditor: React.FC<Props> = ({ node: { addr, details } }) => {
  const {
    actions: { setSettings },
    data: { privateKey },
  } = useAppState()

  const [state, dispatch] = useReducer(reducer, {
    key: 'NotDiverged',
    editor: details.settings,
  })

  useEffect(() => {
    dispatch({ key: 'RemoteUpdated', remote: details.settings })
  }, [details.settings])

  const [validationErrors, setValidationErrors] = useState<ValidationError[]>([])

  const restoreToInitial = () => {
    if (state.key === 'DivergedFromInitial' || state.key === 'DivergedFromBoth') {
      dispatch({ key: 'EditorUpdated', editor: state.initial })
    }
  }
  const updateToLatestRemote = () => {
    if (state.key === 'DivergedFromRemote' || state.key === 'DivergedFromBoth') {
      dispatch({ key: 'EditorUpdated', editor: state.remote })
    }
  }

  const updateSettings = async () => {
    dispatch({ key: 'SavingToRemote', settings: state.editor })
    if (state.editor !== null) {
      await setSettings(addr, toUndefined(privateKey)!, state.editor)
      dispatch({ key: 'Initial', initial: state.editor })
    }
  }

  const onEditorValueChanged = (val: object) => {
    dispatch({ key: 'EditorUpdated', editor: val })
    setValidationErrors(validateAgainstSchema(addr, val, details.settingsSchema))
  }

  const onDirtied = () => {
    setValidationErrors(['Not valid JSON'])
  }

  const valid = validationErrors.length < 1

  return (
    <div className="flex flex-col flex-grow">
      <JsonEditor
        className="border border-gray-200 flex-grow flex-shrink"
        json={state.editor}
        onChanged={onEditorValueChanged}
        onDirtied={onDirtied}
        readOnly={state.key === 'SavingToRemote'}
      />
      <div className="flex-grow-0 flex-shrink-0 pt-4 flex flex-row items-center">
        <Button
          color="blue"
          className={clsx('mr-3')}
          working={state.key === 'SavingToRemote'}
          disabled={state.key === 'SavingToRemote' || state.key === 'NotDiverged' || !valid}
          onClick={updateSettings}
        >
          Save
        </Button>
        <Button
          disabled={!(state.key === 'DivergedFromInitial' || state.key === 'DivergedFromBoth')}
          className={clsx('mr-3')}
          onClick={restoreToInitial}
        >
          Restore
        </Button>
        {(state.key === 'DivergedFromRemote' || state.key === 'DivergedFromBoth') && (
          <Button color="pink" pinging className={clsx('mr-3')} onClick={updateToLatestRemote}>
            Load newest
          </Button>
        )}
        {!(state.key === 'SavingToRemote' || state.key === 'Error') && (
          <div>
            <span
              className={clsx('text-white text-sm font-medium rounded-full px-4 py-2', {
                'bg-green-500': valid,
                'bg-red-500': !valid,
              })}
            >
              {valid ? 'Valid' : 'Invalid'}
            </span>
            {validationErrors.length > 0 && (
              <span className=" ml-2 text-xs text-red-500">{validationErrors.join(', ')}</span>
            )}
          </div>
        )}
      </div>
      {state.key === 'Error' && <div className="pt-3 text-red-500">Error: {state.reason}</div>}
    </div>
  )
}
