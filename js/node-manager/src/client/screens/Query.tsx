import React, { useEffect, useState } from 'react'
import { Layout } from '../components/Layout'
import { useAppState } from '../app-state'
import clsx from 'clsx'
import { Button } from '../components/basics'
import { NodeSelector } from '../components/NodeSelector'
import { useCtrlEnter } from '../components/hooks/use-keycapture'
import AceEditor from 'react-ace'
import 'ace-builds/src-noconflict/mode-sql'
import { EventDiagnostic, Diagnostic, EventResponse } from '../../common/types'
import ReactJson from 'react-json-view'
import { saveToClipboard } from '../util'
import { ClipboardCheckedIcon, ClipboardIcon } from '../components/icons'
import { safeErrorToStr } from '../../common/util'
import { BackgroundColor, BackgroundColorSpectrum } from '../tailwind'
import { NodeManagerAgentContext } from '../agents/node-manager'

type RowProps = {
  accentColor?: BackgroundColorSpectrum
  backgroundColor?: BackgroundColorSpectrum
  textColor?: BackgroundColorSpectrum
  isChecked: boolean
  onChecked?: () => void
  onUnchecked?: () => void
  isFirstRow?: boolean
  expandableObject?: unknown
  children: (onClick: (() => void) | undefined, isExpanded: boolean) => React.ReactNode
  height: RowHeight
  className?: string
  hoverColor?: BackgroundColor
}

export const Row = ({
  accentColor,
  backgroundColor,
  isChecked,
  onChecked,
  onUnchecked,
  isFirstRow,
  expandableObject,
  children,
  textColor,
  height,
  className,
}: RowProps) => {
  const [isExpanded, setIsExpanded] = useState(false)
  const onClick = () => setIsExpanded((c) => !c)
  return (
    <>
      <div
        className={clsx(
          `h-${height} flex flex-row`,
          [backgroundColor && `bg-${backgroundColor}-100`],
          [textColor && `text-${textColor}-600`],
          [isExpanded && accentColor && accentColor !== backgroundColor && `bg-${accentColor}-100`],
          [isExpanded && accentColor && accentColor === backgroundColor && `bg-${accentColor}-200`],
          [!!expandableObject && 'cursor-pointer'],
          {
            'rounded-t-md': isFirstRow,
            'cursor-pointer': !!expandableObject,
          },
          [
            !!expandableObject &&
              accentColor &&
              backgroundColor !== accentColor &&
              `hover:bg-${accentColor}-100`,
          ],
          [
            !!expandableObject &&
              accentColor &&
              accentColor === backgroundColor &&
              `hover:bg-${accentColor}-200`,
          ],
          className,
        )}
      >
        <div
          className={clsx(`h-${height} w-1 flex-shrink-0 border-b`, [
            isExpanded && accentColor && `bg-${accentColor}-300 border-${accentColor}-300`,
          ])}
          onClick={onClick}
        />
        <div
          className={clsx(
            `h-${height} flex-shrink-0 flex-grow-0 px-1 border-r flex items-center w-7 border-b`,
          )}
          onClick={() =>
            isChecked ? (onUnchecked ? onUnchecked() : {}) : onChecked ? onChecked() : {}
          }
        >
          <input
            type="checkbox"
            className={clsx('form-checkbox', {
              'opacity-30': (!onChecked && !isChecked) || (!onUnchecked && isChecked),
            })}
            checked={isChecked}
            readOnly
            disabled={(!onChecked && !isChecked) || (!onUnchecked && isChecked)}
          />
        </div>
        {children(onClick, isExpanded)}
      </div>
      {expandableObject && isExpanded && (
        <div className="border-b">
          {typeof expandableObject === 'object' && expandableObject !== null ? (
            <JsonObject object={expandableObject} accentColor={accentColor} />
          ) : (
            <pre>{JSON.stringify(expandableObject, null, 2)}</pre>
          )}
        </div>
      )}
    </>
  )
}

/** For more values see here https://tailwindcss.com/docs/width */
export type ColWidth = '1' | '16' | '32' | '40' | '44' | '52' | '56' | '96'
const LAMPORT_WIDTH: ColWidth = '16'
const OFFSET_WIDTH: ColWidth = '16'
const TIMESTAMP_WIDTH: ColWidth = '40'
const TAGS_WIDTH: ColWidth = '40'
const APP_WIDTH: ColWidth = '32'

export type RowHeight = '7' | '8' | '9' | '10'
const HEADER_HEIGHT: RowHeight = '8'
const RESULT_HEIGHT: RowHeight = '7'

export type Cell = {
  rowIsExpanded: boolean
  onClick?: () => void
  children: React.ReactNode
  height: RowHeight
  width?: ColWidth
  className?: string
  isLast?: boolean
  backgroundColor?: BackgroundColorSpectrum
}

export const Cell = ({ width, height, onClick, children, className, isLast }: Cell) => {
  return (
    <div
      className={clsx(
        `h-${height}`,
        [width && `w-${width} flex-shrink-0 flex-grow-0`],
        [!width && `flex-grow flex-shrink`],
        [!isLast && 'border-r'],
        'px-1 truncate flex items-center border-b',
        className,
      )}
      onClick={onClick}
    >
      {children}
    </div>
  )
}

export const TruncatableString = ({ children }: { children: React.ReactNode }) => (
  <span className="truncate">{children}</span>
)

const JsonObject = ({
  object,
  accentColor,
}: {
  object: object
  accentColor?: BackgroundColorSpectrum
}) => (
  <div className={clsx('flex flex-row')}>
    <div className={clsx(`w-1 flex-shrink-0`, [accentColor && `bg-${accentColor}-300`])} />
    <div className="text-sm leading-none p-2" id="event-queries-json-viewer">
      <ReactJson
        name={false}
        src={object}
        theme="rjv-default"
        enableClipboard={true}
        displayObjectSize={false}
        displayDataTypes={false}
        collapsed={2}
      />
    </div>
  </div>
)

const HeaderRow = (props: Pick<RowProps, 'isChecked' | 'onChecked' | 'onUnchecked'>) => {
  const cells: [string, ColWidth | undefined][] = [
    ['Lamport', LAMPORT_WIDTH],
    ['Offset', OFFSET_WIDTH],
    ['Timestamp', TIMESTAMP_WIDTH],
    ['Tags', TAGS_WIDTH],
    ['App', APP_WIDTH],
    ['Payload', undefined],
  ]
  return (
    <Row
      height={HEADER_HEIGHT}
      backgroundColor="gray"
      textColor="gray"
      className="font-bold border-t rounded-t-md"
      isFirstRow
      {...props}
    >
      {() =>
        cells.map(([text, width]) => (
          <Cell
            key={text}
            height={HEADER_HEIGHT}
            rowIsExpanded={false}
            width={width}
            isLast={text === 'Payload'}
          >
            <TruncatableString>{text}</TruncatableString>
          </Cell>
        ))
      }
    </Row>
  )
}

export const ResultRow = (
  props: Pick<RowProps, 'isChecked' | 'onChecked' | 'onUnchecked' | 'expandableObject'> & {
    event: EventResponse
  },
) => {
  const cells: [string, string, ColWidth | undefined][] = [
    ['lamport', props.event.lamport.toString(), LAMPORT_WIDTH],
    ['offset', props.event.offset.toString(), OFFSET_WIDTH],
    ['timestamp', new Date(props.event.timestamp / 1000).toISOString(), TIMESTAMP_WIDTH],
    ['tags', props.event.tags.map((t) => `'${t}'`).join(', '), TAGS_WIDTH],
    ['app-id', props.event.appId, APP_WIDTH],
    ['payload', JSON.stringify(props.event.payload), undefined],
  ]
  return (
    <Row height={RESULT_HEIGHT} accentColor="blue" expandableObject={props.event} {...props}>
      {(onClick, rowIsExpanded) =>
        cells.map(([keyPrefix, text, width]) => (
          <Cell
            key={`${keyPrefix}+${text}`}
            height={RESULT_HEIGHT}
            rowIsExpanded={rowIsExpanded}
            width={width}
            onClick={onClick}
            className={clsx({ 'font-mono': keyPrefix === 'payload' })}
            isLast={keyPrefix === 'payload'}
          >
            <TruncatableString>{text}</TruncatableString>
          </Cell>
        ))
      }
    </Row>
  )
}

const DiagnosticRow = (
  props: Pick<RowProps, 'isChecked' | 'onChecked' | 'onUnchecked'> & {
    diagnostic: Diagnostic
  },
) => (
  <Row
    height={RESULT_HEIGHT}
    accentColor={props.diagnostic.severity === 'error' ? 'red' : 'yellow'}
    backgroundColor={props.diagnostic.severity === 'error' ? 'red' : 'yellow'}
    expandableObject={props.diagnostic}
    {...props}
  >
    {(onClick, rowIsExpanded) => (
      <Cell height={RESULT_HEIGHT} rowIsExpanded={rowIsExpanded} onClick={onClick}>
        <TruncatableString>
          {props.diagnostic.severity.toUpperCase()}: {props.diagnostic.message}
        </TruncatableString>
      </Cell>
    )}
  </Row>
)

const isDiagnostics = (event: EventDiagnostic): event is Diagnostic => {
  return (event as Diagnostic).severity !== undefined
}

const Results = ({
  events,
  check,
  uncheck,
  checkAll,
  uncheckAll,
  checkedIxs,
  ixOffset,
  error,
}: {
  events: EventDiagnostic[]
  check: (ix: number) => void
  uncheck: (ix: number) => void
  checkAll: () => void
  uncheckAll: () => void
  checkedIxs: (undefined | true)[]
  ixOffset: number
  error: string
}) =>
  error ? (
    <div className="flex-grow mt-6 border rounded-md mb-1 text-base flex flex-col p-2 text-red-300">
      <pre>{error}</pre>
    </div>
  ) : (
    <div className="flex-grow mt-6 border-b border-l border-r rounded-md mb-1 text-xs flex flex-col">
      <HeaderRow
        isChecked={checkedIxs.length > 0 && checkedIxs.filter((e) => !e).length < 1}
        onChecked={events.length > 0 ? checkAll : undefined}
        onUnchecked={events.length > 0 ? uncheckAll : undefined}
      />
      <div className="flex-grow flex-shrink h-1 overflow-y-scroll overflow-x-hidden">
        {events.map((eventDiagnostic, ix) => {
          const common = {
            isChecked: !!checkedIxs[ix + ixOffset],
            onChecked: () => check(ix + ixOffset),
            onUnchecked: () => uncheck(ix + ixOffset),
          }
          return (
            <React.Fragment key={`row${ix}`}>
              {isDiagnostics(eventDiagnostic) ? (
                <DiagnosticRow {...common} diagnostic={eventDiagnostic} />
              ) : (
                <ResultRow {...common} event={eventDiagnostic} />
              )}
            </React.Fragment>
          )
        })}
        {events.length + ixOffset >= 1000 && (
          <React.Fragment key="lastRow">
            <DiagnosticRow
              isChecked={false}
              diagnostic={{
                severity: 'warning',
                message: 'there may be more rows (Node Manager retrieves at most 1000)',
              }}
            />
          </React.Fragment>
        )}
      </div>
    </div>
  )

const Screen = () => {
  const {
    actions: { setQueryState },
    query: { text: queryStr, node: selectedNodeAddr, results: allEvents },
  } = useAppState()

  const nodeManagerAgent = NodeManagerAgentContext.borrowListen()

  const NUM_EVENTS_PER_PAGE = 250

  const [queryRunning, setQueryRunning] = useState(false)
  const [currentPageIndex, setCurrentPageIndex] = useState(0)
  const [wasSavedToClipboard, setWasSavedToClipboard] = useState(false)
  const [checkedIxs, setCheckedIxs] = useState<(undefined | true)[]>([])
  const [queryError, setQueryError] = useState<string>('')

  const currentEvents = allEvents.slice(currentPageIndex, currentPageIndex + NUM_EVENTS_PER_PAGE)
  const numChecked = checkedIxs.filter((e) => !!e).length

  useEffect(() => {
    let unmounted = false
    if (!wasSavedToClipboard) {
      return
    }
    setTimeout(() => {
      if (!unmounted) {
        setWasSavedToClipboard(false)
      }
    }, 1000)

    return () => {
      unmounted = true
    }
  }, [wasSavedToClipboard])

  const hasNextPage = currentPageIndex + NUM_EVENTS_PER_PAGE >= allEvents.length
  const hasPrevPage = currentPageIndex === 0
  const showNextPage = () => {
    setCurrentPageIndex((curr) => curr + NUM_EVENTS_PER_PAGE)
  }

  const showPrevPage = () => {
    setCurrentPageIndex((curr) => curr - NUM_EVENTS_PER_PAGE)
  }

  const check = (ix: number) => {
    setCheckedIxs((curr) => {
      const n = [...curr]
      n[ix] = true
      return n
    })
  }

  const checkAll = () => {
    setCheckedIxs((curr) => curr.map(() => true))
  }

  const uncheck = (ix: number) => {
    setCheckedIxs((curr) => {
      const n = [...curr]
      n[ix] = undefined
      return n
    })
  }

  const uncheckAll = () => {
    setCheckedIxs([...Array(allEvents.length)])
  }

  const checkedEvents = (): EventDiagnostic[] => {
    const e: EventDiagnostic[] = []
    checkedIxs.forEach((v, ix) => {
      if (v) {
        e.push(allEvents[ix])
      }
    })
    return e
  }

  const toClipboard = () => {
    saveToClipboard(JSON.stringify(checkedEvents(), null, 2))
    setWasSavedToClipboard(true)
  }

  const runQueryDisabled = !selectedNodeAddr || !queryStr

  const runQuery = async () => {
    if (runQueryDisabled) return

    setQueryRunning(true)
    setQueryState((s) => ({ ...s, results: [] }))
    setCurrentPageIndex(0)
    try {
      const { events } = await nodeManagerAgent.api.query({
        addr: selectedNodeAddr,
        query: queryStr,
      })
      if (!events) {
        console.log(`node doesn't support querying`)
        setQueryRunning(false)
        setQueryError("This node doesn't support queries. Please update to Actyx 2.2.0 or later.")
        return
      }
      setQueryRunning(false)
      setCheckedIxs([...Array(events.length)])
      setQueryState((s) => ({ ...s, results: events }))
      setQueryError('')
    } catch (error) {
      console.error(error)
      setQueryRunning(false)
      setQueryError(safeErrorToStr(error))
    }
  }

  // Ctrl+Enter to submit
  useCtrlEnter(runQuery)

  return (
    <Layout title={`Query`}>
      <div className="bg-white rounded p-4 min-h-full w-full min-w-full max-w-full overflow-hidden flex flex-col items-stretch h-full">
        <div className="flex-grow flex flex-row h-full">
          {/* DON'T REMOVE THIS; MAY BE USEFUL IN THE FUTURE FOR A LEFT SIDEBAR
          <div className="w-56 border-r pr-1 mr-5 flex-grow-0 flex-shrink-0 overflow-auto flex flex-col">
            <p className="flex-grow-0">Queries</p>
            <div className="flex-grow overflow-y-auto">
              <p>Left</p>
            </div>
          </div> */}
          <div className="flex-grow flex-shrink flex flex-col max-w-full">
            <div>
              <AceEditor
                readOnly={false}
                className="w-full border rounded-md"
                mode="sql"
                theme="textmate"
                name="event-query"
                onChange={(t) => setQueryState((s) => ({ ...s, text: t }))}
                fontSize={18}
                showPrintMargin={false}
                height={`120px`}
                width={`100%`}
                showGutter={false}
                highlightActiveLine={true}
                minLines={100}
                value={queryStr}
                setOptions={{
                  showFoldWidgets: false,
                  showLineNumbers: true,
                  tabSize: 2,
                  useWorker: false,
                }}
              />
              <div className="flex flex-row pt-3 justify-stretch items-stretch gap-3">
                <NodeSelector
                  nodes={nodeManagerAgent.api.getReachableUiNodes()}
                  selectedNodeAddr={selectedNodeAddr}
                  onChange={(v) =>
                    v
                      ? setQueryState((s) => ({ ...s, node: v.value }))
                      : setQueryState((s) => {
                          const { node, ...rest } = s
                          return rest
                        })
                  }
                />

                <Button
                  color="blue"
                  disabled={runQueryDisabled}
                  onClick={runQuery}
                  working={queryRunning}
                >
                  Query
                </Button>
              </div>
            </div>
            <Results
              events={currentEvents}
              check={check}
              uncheck={uncheck}
              checkAll={checkAll}
              uncheckAll={uncheckAll}
              ixOffset={currentPageIndex}
              checkedIxs={checkedIxs}
              error={queryError}
            />
            <div className="flex flex-grow-0 flex-row justify-end pt-3">
              <div className="flex-grow flex-shrink">
                {allEvents.length > 0 && (
                  <>
                    <span>
                      Showing {currentPageIndex + 1} to{' '}
                      {Math.min(allEvents.length, currentPageIndex + NUM_EVENTS_PER_PAGE)} of{' '}
                      {allEvents.length} events
                      {allEvents.length >= 1000 ? ' (Node Manager retrieves at most 1000)' : ''}.
                    </span>
                    <Button
                      className="ml-3"
                      color="gray"
                      small
                      outline
                      disabled={hasPrevPage}
                      onClick={showPrevPage}
                    >
                      Previous
                    </Button>
                    <Button
                      className="ml-1"
                      color="gray"
                      small
                      outline
                      disabled={hasNextPage}
                      onClick={showNextPage}
                    >
                      Next
                    </Button>
                  </>
                )}
                {numChecked > 0 && (
                  <span className="ml-3 text-gray-300 italic">
                    {numChecked} of {allEvents.length} events selected.
                  </span>
                )}
              </div>
              <Button
                className="ml-1"
                onClick={toClipboard}
                disabled={numChecked < 1}
                icon={!wasSavedToClipboard ? <ClipboardIcon /> : <ClipboardCheckedIcon />}
              >
                Copy
              </Button>
            </div>
          </div>
        </div>
      </div>
    </Layout>
  )
}

export default Screen
