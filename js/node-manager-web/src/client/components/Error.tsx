import React from 'react'
import { ClassName } from '../react'
import clsx from 'clsx'

export const Error: React.FC<ClassName> = ({ className, children }) => (
  <div className={clsx('p-12 w-full h-full flex justify-center items-center', className)}>
    <div className="bg-red-200 rounded shadow p-12 text-center border border-red-300">
      <p className="text-red-400 uppercase font-medium text-sm mb-2">Error</p>
      {children}
    </div>
  </div>
)
