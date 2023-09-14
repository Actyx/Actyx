import * as E from 'fp-ts/Either'
import React, { useEffect, useMemo, useRef, useState } from 'react'
import AceEditor from 'react-ace'
import { Button } from '../components/basics'
import { Layout } from '../components/Layout'
import { NodeSelector } from '../components/NodeSelector'
import { useAppState, Actions } from '../app-state'
import 'ace-builds/src-noconflict/mode-json'
import 'ace-builds/src-noconflict/mode-text'
import { PublishResponse } from 'common/types'

const Screen = () => {
  const {
    data: { nodes },
    actions: { setPublishState, publish },
    publish: { node: selectedNodeAddr, tagsField, payloadField },
  } = useAppState()

  const [payloadErrorMessage, setPayloadErrorMessage] = useState<null | string>(null)
  const [lastResult, setLastResult] = useState<null | E.Either<string, PublishResponse>>(null)
  const [isPublishing, setIsPublishing] = useState(false)

  const payloadErrorDebounce = useDebouncer()

  const tags = tagsField
    .split(',')
    .map((x) => x.trim())
    .filter((x) => x)

  useEffect(() => {
    payloadErrorDebounce.register(() => {
      setPayloadErrorMessage(() => {
        const maybeJSON = payloadField.trim()
        if (!maybeJSON) return null
        const result = verifyJSON(payloadField)
        if (E.isRight(result)) return null
        return String(result.left)
      })
    }, 800)
  }, [payloadField])

  const publishButtonDisabled = !selectedNodeAddr || tags.length === 0
  const publishButtonFn = !selectedNodeAddr
    ? undefined
    : async () => {
        const promise = publishImpl({
          payloadField,
          publishFn: publish,
          selectedNodeAddr,
          tags,
        })

        if (!promise) return

        setIsPublishing(true)
        const result = await promise
        setPublishState((prev) => ({ ...prev, tagsField: '', payloadField: '' }))
        setPayloadErrorMessage(null)
        setLastResult(result)
        setIsPublishing(false)
      }

  return (
    <Layout title={`Publish`}>
      <div className="bg-white rounded p-4 min-h-full w-full min-w-full max-w-full overflow-hidden flex flex-col items-stretch h-full">
        <div className="pt-3 z-10">
          <NodeSelector
            nodes={nodes}
            selectedNodeAddr={selectedNodeAddr}
            onChange={(node) =>
              setPublishState((prev) => ({ ...prev, node: node?.value || undefined }))
            }
          />
        </div>
        <div className="pt-3">
          <AceEditor
            {...TAGS_EDITOR_CONFIG}
            className="w-full border rounded-md"
            onChange={(val) => setPublishState((prev) => ({ ...prev, tagsField: val }))}
            placeholder="Tags | comma delimited | e.g. 'created,started,working,finished'"
            width={`100%`}
            value={tagsField}
          />
        </div>
        <div className="pt-3">
          <AceEditor
            {...PAYLOAD_EDITOR_CONFIG}
            className="w-full border rounded-md"
            placeholder={`Payload | JSON | e.g. '{ "foo": "bar" }'`}
            onChange={(val) => setPublishState((prev) => ({ ...prev, payloadField: val }))}
            height={`120px`}
            width={`100%`}
            value={payloadField}
          />
        </div>
        <div className="pt-3">
          <Button
            color="blue"
            disabled={publishButtonDisabled}
            onClick={publishButtonFn}
            working={isPublishing}
          >
            Publish
          </Button>
        </div>
        <div className="pt-3">
          <ResultReport payloadErrorMessage={payloadErrorMessage} result={lastResult} />
        </div>
      </div>
    </Layout>
  )
}

const ResultReport = ({
  result,
  payloadErrorMessage,
}: {
  result: null | E.Either<string, PublishResponse>
  payloadErrorMessage: null | string
}) => {
  const successfulResult =
    (result && E.isRight(result) && result.right.data[0] && JSON.stringify(result.right.data[0])) ||
    undefined
  const failedResult = result && E.isLeft(result) && JSON.stringify(result.left)

  return (
    <div>
      {payloadErrorMessage && <div className="text-yellow-600">{payloadErrorMessage}</div>}
      {successfulResult && (
        <div className="text-lime-600">Publish Successful {successfulResult}</div>
      )}
      {failedResult && <div className="text-red-600">Error: {failedResult}</div>}
    </div>
  )
}

export default Screen

// ==========
// Utilities
// ==========

const TAGS_EDITOR_CONFIG = {
  readOnly: false,
  mode: 'text',
  theme: 'textmate',
  name: 'tags',
  fontSize: 18,
  showPrintMargin: false,
  showGutter: false,
  highlightActiveLine: false,
  minLines: 1,
  maxLines: 1,
}

const PAYLOAD_EDITOR_CONFIG = {
  readOnly: false,
  mode: 'json',
  theme: 'textmate',
  name: 'payload',
  fontSize: 18,
  showPrintMargin: false,
  showGutter: false,
  highlightActiveLine: true,
  setOptions: {
    showFoldWidgets: true,
    showLineNumbers: true,
    tabSize: 2,
    useWorker: false,
  },
}

const verifyJSON = (str: string) =>
  E.tryCatch(
    () => JSON.parse(str),
    (e) => e,
  )

const publishImpl = ({
  publishFn,
  tags,
  selectedNodeAddr,
  payloadField,
}: {
  publishFn: Actions['publish']
  payloadField: string
  selectedNodeAddr: string
  tags: string[]
}): undefined | Promise<E.Either<string, PublishResponse>> => {
  if (!selectedNodeAddr) return
  if (tags.length === 0) return
  const payloadRes = verifyJSON(payloadField)

  if (E.isLeft(payloadRes)) return
  const payload = payloadRes.right

  return publishFn({
    addr: selectedNodeAddr,
    events: [{ tags, payload }],
  })
    .then((x) => E.right(x))
    .catch((e) => E.left(String(e)))
}

// ==========
// Hooks
// ==========

const makeDebouncer = () => {
  let storedTimeout: number | undefined = undefined

  const clear = () => clearTimeout(storedTimeout)

  const register = (fn: Function, timeout: number) => {
    clear()
    storedTimeout = setTimeout(fn, timeout) as unknown as number
  }

  return { register, clear }
}

const useDebouncer = () => {
  const inner = useMemo(makeDebouncer, [])

  useEffect(() => {
    // clear when exit
    return () => {
      inner.clear()
    }
  }, [inner])

  return inner
}
