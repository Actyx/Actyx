import React, { useState } from 'react'
import clsx from 'clsx'
import { ClassName } from '../../react'

const Rider: React.FC<{
  active: boolean
  text: string
  onClick: () => void
}> = ({ active, text, onClick }) => (
  <li
    onClick={onClick}
    className={clsx(
      'px-4 py-2 border-b-2 border-gray-300 font-semibold text-gray-800 rounded-t opacity-50 cursor-pointer',
      {
        'border-gray-600 bg-gray-300': active,
        'hover:bg-gray-300 hover:border-b-4 hover:border-gray-600': !active,
      },
    )}
  >
    {text}
  </li>
)
const Content: React.FC<ClassName & { elem: JSX.Element; hidden: boolean }> = ({
  elem,
  hidden,
  className,
}) => (
  <div hidden={hidden} className={clsx('', className)}>
    {elem}
  </div>
)

interface Props {
  tabs: {
    text: string
    elem: JSX.Element
  }[]
  contentClassName?: string
}

const E: React.FC<Props & ClassName> = ({ tabs, className, contentClassName }) => {
  const [current, setCurrent] = useState(0)
  return (
    <div className={clsx('w-full flex flex-col rounded flex-grow flex-shrink', className)}>
      <ul className="flex-grow-0 flex-shrink-0 inline-flex w-full pb-4">
        {tabs.map(({ text }, i) => (
          <Rider text={text} key={text + i} onClick={() => setCurrent(i)} active={current === i} />
        ))}
      </ul>

      <div className="flex-grow flex-shrink flex flex-col items-stretch">
        {tabs.map(({ elem }, i) => (
          <Content
            className={clsx(
              { flex: current === i },
              'flex-col flex-grow flex-shrink',
              contentClassName,
            )}
            key={'tab_content' + i}
            elem={elem}
            hidden={current !== i}
          />
        ))}
      </div>
    </div>
  )
}

export default E
