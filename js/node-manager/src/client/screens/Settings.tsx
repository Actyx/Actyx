import { useAppState } from '../app-state'
import { Button } from '../components/basics'
import { Input, Layout } from '../components/Layout'
import React, { useEffect, useMemo, useState } from 'react'
import AceEditor from 'react-ace'
import clsx from 'clsx'
import { NodeType, ReachableNodeUi } from '../../common/types'
import { get, set, parse } from 'json-pointer'
import deepEqual from 'deep-equal'
import { validateAgainstSchema } from '../components/SettingsEditor'

const Screen: React.FC<{}> = () => {
  /**
   * The idea is the following:
   * - path & json are in app state to allow navigating to other sections without losing editor state
   * - when changing the path, wipe all editor state and start from current common settings
   * - when coming back from other section, keep path & json last edited
   * This is done by setting json=null upon path change, signaling that not json but common should be
   * placed in the editor.
   */
  const {
    settings: { path, json },
    actions: { setSettingJson, setSettingPath, setSettings },
    data: { nodes },
  } = useAppState()
  const [accept, setAccept] = useState(0)
  const [reject, setReject] = useState(0)
  const [timer, setTimer] = useState<ReturnType<typeof setTimeout>>()
  const [writing, setWriting] = useState(false)
  const [rejected, setRejected] = useState<string[]>([])
  const [errors, setErrors] = useState<Set<string>>(new Set())

  const [common, invalid] = useMemo(() => {
    let pointer: string[] = []
    try {
      pointer = parse(path)
    } catch (e) {
      console.error(`cannot parse pointer ${path}:`, e)
      return [undefined, true]
    }
    const c = nodes
      .filter((x): x is ReachableNodeUi => x.type === NodeType.Reachable)
      .reduce((acc, node) => {
        if (acc === undefined) return acc
        try {
          const found = JSON.stringify(get(node.details.settings, pointer), undefined, 2)
          return acc === '' ? found : acc === found ? acc : undefined
        } catch (e) {
          return acc
        }
      }, '' as string | undefined)
    return [c, false]
  }, [path, nodes])

  const setPath = (p: string) => {
    if (p === path) return
    setSettingPath(p)
    setSettingJson(null)
  }

  const validate = (j?: string | null) => {
    const jso = j === undefined ? json : j
    setTimer(undefined)
    setRejected([])
    setErrors(new Set())
    if (jso === null) {
      setAccept(0)
      setReject(0)
      return
    }
    let val: unknown
    try {
      val = JSON.parse(jso)
    } catch (e) {
      setErrors(new Set([`${e}`]))
      setAccept(0)
      setReject(nodes.length)
      return
    }
    let a = 0
    let r = 0
    const e: string[] = []
    for (const node of nodes) {
      if (node.type !== NodeType.Reachable) continue
      const { settings, settingsSchema } = node.details
      const s: object = JSON.parse(JSON.stringify(settings))
      set(s, path, val)
      const res = validateAgainstSchema(s, settingsSchema)
      if (res.length === 0) a += 1
      else {
        r += 1
        e.push(...res)
      }
    }
    setAccept(a)
    setReject(r)
    setErrors(new Set(e))
  }

  /* eslint-disable-next-line react-hooks/exhaustive-deps */
  useEffect(validate, [path])

  const setJson = (j: string) => {
    if (j === json) return
    setSettingJson(j)
    setTimer((current) => {
      if (current !== undefined) clearTimeout(current)
      return setTimeout(() => validate(j), 500)
    })
  }

  const apply = async () => {
    setWriting(true)
    const res = await Promise.all(
      nodes.map(async (node) => {
        if (node.type !== NodeType.Reachable || json === null) return
        try {
          await setSettings(node.addr, JSON.parse(json), parse(path))
          return
        } catch {
          return node.addr
        }
      }),
    )
    console.log('applied new settings')
    setWriting(false)
    const rejected = res.filter((a): a is string => typeof a === 'string')
    if (rejected.length) {
      setRejected(rejected)
    } else {
      setSettingJson(null)
      validate(null)
    }
  }

  const discard = () => {
    setSettingJson(null)
    validate(null)
  }

  return (
    <Layout title="Swarm Settings" flex>
      <div>
        <input
          type="text"
          placeholder="settings scope"
          value={path}
          className={clsx(
            'text-sm w-96 rounded-md bg-white border-transparent focus:border-gray-300 focus:ring-0',
            invalid && 'bg-red-200',
          )}
          onChange={(e) => setPath(e.target.value)}
        />
        &nbsp;{' '}
        <span className="text-gray-500 italic">
          (
          {common === ''
            ? 'no values'
            : common === undefined
            ? 'multiple values'
            : 'one common value'}{' '}
          at this scope)
        </span>
      </div>
      <AceEditor
        readOnly={writing}
        mode="json"
        theme="textmate"
        name={`settings-${path}`}
        onChange={setJson}
        className="border border-gray-200 flex-grow my-8"
        height="auto"
        width="auto"
        fontSize={14}
        showPrintMargin={false}
        showGutter={false}
        highlightActiveLine={true}
        value={json !== null ? json : common}
        setOptions={{
          showFoldWidgets: false,
          showLineNumbers: true,
          tabSize: 2,
          useWorker: false,
        }}
        onLoad={(editor) => editor.resize()}
      />
      {errors.size > 0 ? (
        <ul className="mb-4 text-red-500 list-disc list-inside">
          {[...errors].map((e) => (
            <li key={e}>{e}</li>
          ))}
        </ul>
      ) : undefined}
      {rejected.length > 0 ? (
        <div className="mb-4 text-yellow-500">
          <p>New settings were rejected by the following nodes:</p>
          <ul className="list-disc list-inside">
            {rejected.map((addr) => (
              <li key={addr}>
                <code>{addr}</code>
              </li>
            ))}
          </ul>
        </div>
      ) : undefined}
      <div>
        <Button disabled={accept === 0} onClick={apply}>
          Set on all Nodes
        </Button>
        <Button disabled={json === null} onClick={discard}>
          Discard changes
        </Button>
        &nbsp; (validation status:{' '}
        {timer === undefined ? (
          <>
            <span className={clsx(reject > 0 ? 'text-yellow-500' : 'text-green-500')}>
              {reject} reject
            </span>
            ,{' '}
            <span className={clsx(accept === 0 ? 'text-red-500' : 'text-green-500')}>
              {accept} accept
            </span>
          </>
        ) : (
          <span className="italic text-gray-500">validating</span>
        )}
        )
      </div>
    </Layout>
  )
}

export default Screen
