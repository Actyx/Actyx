import React from 'react'
import clsx from 'clsx'
import { BackgroundColor } from '../tailwind'

export const SimpleCanvas: React.FC<{ bgColor?: BackgroundColor }> = ({ bgColor, children }) => {
  const bgColorClass = `bg-${bgColor || 'white'}`
  return (
    <div
      className={clsx(
        'rounded p-4 min-h-full w-full min-w-full flex flex-col items-stretch',
        bgColorClass,
      )}
    >
      {children}
    </div>
  )
}
