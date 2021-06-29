import React from 'react'
import { ClassName } from '../../react'
import clsx from 'clsx'
import { Button } from './'

interface InputProps {
  disabled?: boolean
  label: string
  placeholder?: string
  value: string
  setValue?: (s: string) => void
  inputClassName?: string
  button?: {
    text: string
    onClick: () => void
    disabled?: boolean
  }
}

const Input: React.FC<InputProps & ClassName> = ({
  disabled,
  label,
  placeholder,
  value,
  setValue,
  className,
  inputClassName,
  button,
}) => (
  <div className={clsx(className)}>
    <label htmlFor={label} className="block text-sm font-medium text-gray-500 mb-1">
      {label}
    </label>
    <div className="flex">
      <input
        type="text"
        name={label}
        className={clsx(
          'focus:ring-indigo-500 focus:border-indigo-500 block w-full border-gray-300 rounded',
          {
            'bg-gray-100': disabled,
          },
          inputClassName,
        )}
        placeholder={placeholder}
        value={value}
        // eslint-disable-next-line @typescript-eslint/no-empty-function
        onChange={setValue ? (e) => setValue(e.target.value) : () => {}}
        disabled={disabled}
      />
      {button && (
        <Button className="ml-2 flex-shrink-0" onClick={button.onClick} disabled={button.disabled}>
          {button.text}
        </Button>
      )}
    </div>
  </div>
)

export default Input
