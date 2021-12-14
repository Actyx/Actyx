import { SimpleCanvas } from '../components/SimpleCanvas'
import React, { useState } from 'react'
import { Layout } from '../components/Layout'
import { useStore } from '../store'
import clsx from 'clsx'
import { StoreStateKey } from '../store/types'
import { DEFAULT_TIMEOUT_SEC } from 'common/consts'

const Analytics = () => {
  const store = useStore()
  const checked = store.key === StoreStateKey.Loaded && store.data.analytics.disabled
  const onChange = (isChecked: boolean) => {
    if (store.key === StoreStateKey.Loaded) {
      store.actions.updateAndReload({
        ...store.data,
        analytics: {
          ...store.data.analytics,
          disabled: isChecked,
        },
      })
    }
  }
  return (
    <label className="inline-flex items-center p-1">
      <span>Disable anonymous aggregate user behaviour analytics:</span>
      <input
        className="ml-2"
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
    </label>
  )
}

const NodeTimeout = () => {
  const store = useStore()
  const current =
    store.key === StoreStateKey.Loaded && store.data.preferences.nodeTimeout !== undefined
      ? store.data.preferences.nodeTimeout.toString()
      : ''
  const [formValue, setFormValue] = useState<string>(current)
  const isValid = formValue !== '' && !isNaN(parseInt(formValue)) && parseInt(formValue) > 0
  const isError = formValue !== '' && !isValid
  const isDiff = formValue !== current && formValue && DEFAULT_TIMEOUT_SEC.toString()

  if (store.key !== StoreStateKey.Loaded) {
    return <p>Loading...</p>
  }

  const onSave = () => {
    if (!isValid || store.key !== StoreStateKey.Loaded) {
      return
    }
    store.actions.updateAndReload({
      ...store.data,
      preferences: {
        ...store.data.preferences,
        nodeTimeout: parseInt(formValue),
      },
    })
  }

  const onReset = () => {
    if (store.key !== StoreStateKey.Loaded) {
      return
    }
    store.actions.updateAndReload({
      ...store.data,
      preferences: {
        ...store.data.preferences,
        nodeTimeout: undefined,
      },
    })
    setFormValue('')
  }
  return (
    <div className="inline-flex items-center p-1">
      <label>Node connectivity timeout (seconds):</label>
      <input
        type="number"
        min="1"
        className={clsx(`ml-2 pl-2 w-16 pr-0 py-0 border-1 outline-none focus:outline-none`, {
          'border-red-500': isError,
        })}
        onChange={(event) => setFormValue(event.target.value)}
        value={formValue}
        placeholder={DEFAULT_TIMEOUT_SEC.toString()}
      />
      {isDiff && (
        <span onClick={onSave} className="ml-2 underline text-blue-500 cursor-pointer">
          save
        </span>
      )}
      {!isDiff && current !== undefined && (
        <span onClick={onReset} className="ml-2 underline text-blue-500 cursor-pointer">
          reset
        </span>
      )}
    </div>
  )
}

const Screen: React.FC<{}> = () => {
  return (
    <Layout title="Preferences">
      <SimpleCanvas>
        <div className="flex flex-col flex-grow flex-shrink">
          <p className="text-gray-400 pb-3 flex-grow-0 flex-shrink-0">
            Configure the Node Manager to fit your workflow.
          </p>
          <Analytics />
          <NodeTimeout />
        </div>
      </SimpleCanvas>
    </Layout>
  )
}

export default Screen
