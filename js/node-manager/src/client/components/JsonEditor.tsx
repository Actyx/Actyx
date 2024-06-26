import React, { useEffect, useState } from 'react'
import AceEditor from 'react-ace'
import 'ace-builds/src-noconflict/mode-json'
import 'ace-builds/src-noconflict/theme-textmate'
import { ClassName } from '../react'
import clsx from 'clsx'
import eq from 'deep-equal'

interface Props {
  json?: object | null
  onChanged: (json: object) => void
  onDirtied?: () => void
  readOnly?: boolean
}

const toStr = (json: object | null | undefined, format?: boolean): string =>
  json ? JSON.stringify(json, null, format ? 2 : 0) : ''

export const JsonEditor: React.FC<Props & ClassName> = ({
  json,
  onChanged,
  onDirtied,
  className,
  readOnly,
}) => {
  const [str, setStr] = useState(toStr(json, true))
  useEffect(() => {
    setStr((curr) => {
      try {
        const currentObj = JSON.parse(curr)
        if (!eq(json, currentObj)) {
          return toStr(json, true)
        } else {
          return curr
        }
      } catch (error) {
        return toStr(json, true)
      }
    })
    //setStr(toStr(json, true))
  }, [json])

  const onChange = (val: string) => {
    try {
      const parsed = JSON.parse(val)
      setStr(val)
      onChanged(parsed)
    } catch (error) {
      if (onDirtied) {
        onDirtied()
      }
      setStr(val)
    }
  }

  return (
    <AceEditor
      readOnly={readOnly}
      mode="json"
      theme="textmate"
      name="blah2"
      onChange={onChange}
      className={className}
      height="auto"
      width="auto"
      fontSize={14}
      showPrintMargin={false}
      showGutter={false}
      highlightActiveLine={true}
      value={str}
      setOptions={{
        showFoldWidgets: false,
        showLineNumbers: true,
        tabSize: 2,
        useWorker: false,
      }}
      onLoad={(editor) => editor.resize()}
    />
  )
}
