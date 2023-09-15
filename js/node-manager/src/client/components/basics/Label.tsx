import clsx from 'clsx'
import * as React from 'react'

const Label = ({
  className,
  ...props
}: React.DetailedHTMLProps<React.LabelHTMLAttributes<HTMLLabelElement>, HTMLLabelElement>) => (
  <label {...props} className={clsx('block text-sm font-medium text-gray-500 mb-1', className)} />
)

export default Label
