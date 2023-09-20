import * as E from 'fp-ts/Either'
import React, { useEffect, useState } from 'react'
import AceEditor from 'react-ace'
import { Button, Label } from '../components/basics'
import { Layout } from '../components/Layout'
import { NodeSelector } from '../components/NodeSelector'
import { useDebouncer } from '../components/hooks/use-debouncer'
import { useCtrlEnter } from '../components/hooks/use-keycapture'
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

  // Check for JSON format errors
  useEffect(() => {
    payloadErrorDebounce.register(() => {
      setPayloadErrorMessage(() => {
        const maybeJSON = payloadField.trim()
        if (!maybeJSON) return null
        const result = verifyJSON(payloadField)
        if (E.isRight(result)) return null
        return String(result.left)
      })
    }, 500)
  }, [payloadField])

  // Ctrl+Enter to submit
  useCtrlEnter(publishButtonFn)

  return (
    <Layout title={`Publish`}>
      <div className="bg-white rounded p-4 min-h-full w-full min-w-full max-w-full overflow-hidden flex flex-col items-stretch h-full gap-3">
        <div>
          <Label htmlFor={TAGS_EDITOR_CONFIG.name}>Tags (comma delimited)</Label>
          <AceEditor
            {...TAGS_EDITOR_CONFIG}
            className="w-full border rounded-md"
            onChange={(val) =>
              setPublishState((prev) => ({ ...prev, tagsField: stripNewLines(val) }))
            }
            placeholder="created,started,working,finished"
            width={`100%`}
            value={tagsField}
          />
        </div>
        <div>
          <Label htmlFor={PAYLOAD_EDITOR_CONFIG.name}>Payload (JSON)</Label>
          <AceEditor
            {...PAYLOAD_EDITOR_CONFIG}
            className="w-full border rounded-md"
            placeholder={PAYLOAD_PLACEHOLDER}
            onChange={(val) => setPublishState((prev) => ({ ...prev, payloadField: val }))}
            height={`120px`}
            width={`100%`}
            value={payloadField}
          />
        </div>
        <div className="z-10 flex flex-row justify-stretch items-stretch gap-3">
          <NodeSelector
            nodes={nodes}
            selectedNodeAddr={selectedNodeAddr}
            onChange={(node) =>
              setPublishState((prev) => ({ ...prev, node: node?.value || undefined }))
            }
          />
          <Button
            color="blue"
            disabled={publishButtonDisabled}
            onClick={publishButtonFn}
            working={isPublishing}
          >
            Publish
          </Button>
        </div>
        <div>
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
  const successMessage =
    (result && E.isRight(result) && result.right.data[0] && JSON.stringify(result.right.data[0])) ||
    undefined
  const failedMessage = result && E.isLeft(result) && JSON.stringify(result.left)

  return (
    <div>
      {payloadErrorMessage && <div className="text-yellow-600">{payloadErrorMessage}</div>}
      {successMessage && <div className="text-lime-600">Publish Successful {successMessage}</div>}
      {failedMessage && <div className="text-red-600">Error: {failedMessage}</div>}
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

const PAYLOAD_PLACEHOLDER = `{
  "foo": "bar"
}`

const stripNewLines = (str: string) => str.replace(/\n/gi, '')

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
