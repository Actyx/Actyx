import React, { useState } from 'react'
import clsx from 'clsx'
import {
  BarChartIcon,
  Cube3DIcon,
  DocumentIcon,
  SignatureIcon,
  HelpIcon,
  HomeIcon,
  InboxIcon,
  PreferencesIcon,
  ShieldIcon,
  VerticalTripleDots,
  NodeManagerIcon,
  KeyIcon,
  ClipboardIcon,
  ClipboardCheckedIcon,
} from './icons'
import { ClassName } from '../react'
import { getNodesDetails } from '../util'
import { useAppState, AppActionKey, AppStateKey } from '../app-state'

// Inspiration: https://codepen.io/robstinson/pen/zYBGNQB
const NavButton: React.FC<{
  onClick: () => void
  text: string
  icon: JSX.Element
  bottom?: boolean
  active?: boolean
  hidden?: boolean
}> = ({ hidden, children, onClick, bottom, text, icon, active }) => (
  <div
    className={clsx('px-2 w-full', {
      'mt-auto': bottom,
    })}
    hidden={hidden}
  >
    <button
      className={clsx(
        'flex items-center justify-left px-2 flex-shrink-0 w-full h-10 mt-2 rounded hover:bg-gray-300 focus:outline-none',
        {
          'bg-gray-300': active,
        },
      )}
      onClick={onClick}
    >
      {icon}
      <span className="ml-2">{text}</span>
    </button>
  </div>
)

interface InputProps {
  placeholder?: string
  onSubmit: (value: string) => void
  buttonText?: string
  validator?: (value: string) => boolean
  transformer?: (value: string) => string
}

export const Input: React.FC<InputProps & ClassName> = ({
  className,
  placeholder,
  buttonText,
  onSubmit,
  validator,
  transformer,
}) => {
  const [val, setVal] = useState('')
  const doSubmit = () => {
    if (!validator || validator(val)) {
      onSubmit(val)
      setVal('')
    }
  }
  const onKeyUp = (event: React.KeyboardEvent) => {
    if (event.key === 'Enter') {
      doSubmit()
    }
  }
  return (
    <>
      <input
        type="text"
        placeholder={placeholder}
        className={clsx(
          'block text-sm w-60 rounded-md bg-white border-transparent focus:border-gray-300 focus:ring-0',
          className,
          {
            'border-red-500 focus:border-red-500': !!(val && validator && !validator(val)),
          },
        )}
        onKeyUp={onKeyUp}
        value={val}
        onChange={(e) => setVal(transformer ? transformer(e.target.value) : e.target.value)}
      />
      {buttonText && (
        <button
          className="flex items-center justify-center h-10 px-4 ml-2 text-sm font-medium rounded bg-gray-200 hover:bg-gray-300 focus:outline-none"
          onClick={doSubmit}
          disabled={!!(val && validator && !validator(val))}
        >
          {buttonText}
        </button>
      )}
    </>
  )
}
interface ActionProps {
  text: string
  target: (() => void) | string
  disabled?: boolean
}
export const Action: React.FC<ActionProps & ClassName> = ({ className, text, target, disabled }) =>
  typeof target === 'function' ? (
    <button
      className={clsx(
        'flex items-center justify-center h-10 px-4 ml-2 text-sm font-medium rounded hover:bg-gray-300 focus:outline-none',
        {
          'cursor-not-allowed': disabled,
        },
        className,
      )}
      onClick={target}
      disabled={disabled}
    >
      {text}
    </button>
  ) : (
    <a
      className={clsx(
        'flex items-center justify-center h-10 px-4 ml-2 text-sm font-medium rounded hover:bg-gray-300 focus:outline-none',
        className,
      )}
      href={target}
      target="_blank"
      rel="noopener noreferrer"
    >
      {text}
    </a>
  )

interface MenuItem {
  text: string
  target: (() => void) | string
}

interface MenuProps {
  items: MenuItem[]
}
export const Menu: React.FC<MenuProps & ClassName> = ({ className, items }) => {
  const [open, setOpen] = useState(false)

  return (
    <button
      className={clsx('relative ml-2 text-sm focus:outline-none group', className)}
      onClick={() => setOpen((current) => !current)}
    >
      <div className="flex items-center justify-between w-10 h-10 rounded hover:bg-gray-300">
        <svg
          className="w-5 h-5 mx-auto"
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <VerticalTripleDots className="mx-auto" />
        </svg>
      </div>
      <div
        className={clsx(
          'absolute right-0 flex flex-col items-start w-44 pb-1 bg-white border border-gray-300 shadow-lg group-focus:flex',
          {
            hidden: !open,
          },
        )}
      >
        {items.map(({ text, target }, i) =>
          typeof target === 'function' ? (
            <button
              key={text + i}
              className="w-full px-4 py-2 text-left hover:bg-gray-300"
              onClick={target}
            >
              {text}
            </button>
          ) : (
            <a
              key={text + i}
              className="w-full px-4 py-2 text-left hover:bg-gray-300"
              href={target}
              target="_blank"
              rel="noopener noreferrer"
            >
              {text}
            </a>
          ),
        )}
      </div>
    </button>
  )
}
interface LayoutProps {
  title?: string
  input?: InputProps
  actions?: ActionProps[]
  menuItems?: MenuItem[]
}

export const Layout: React.FC<LayoutProps & ClassName> = ({
  title,
  children,
  className,
  input,
  actions,
  menuItems,
}) => {
  const { state, dispatch } = useAppState()

  const hideMenuItems = state.key === AppStateKey.SetupUserKey

  const menuProps: MenuProps = {
    items: [
      {
        target: 'https://community.actyx.com/',
        text: 'Get help',
      } as MenuItem,
      {
        target: 'https://www.actyx.com/contact',
        text: 'Contact Actyx',
      } as MenuItem,
    ].concat(menuItems || []),
  }

  return (
    <div className={clsx('flex w-screen h-screen text-gray-700', className)}>
      <div className="flex flex-col items-center w-44 pb-4 overflow-auto border-r border-gray-300 flex-shrink-0">
        <div className="flex items-center justify-center mb-4 flex-shrink-0 w-full h-16 bg-gray-100">
          <NodeManagerIcon width={12} height={12} />
        </div>
        <NavButton
          onClick={() => dispatch({ key: AppActionKey.ShowOverview })}
          icon={<InboxIcon />}
          text="Nodes"
          active={state.key === AppStateKey.Overview || state.key === AppStateKey.NodeDetail}
          hidden={hideMenuItems}
        />
        <NavButton
          onClick={() => dispatch({ key: AppActionKey.ShowNodeAuth })}
          icon={<ShieldIcon />}
          text="Node Auth"
          active={state.key === AppStateKey.NodeAuth}
          hidden={hideMenuItems}
        />
        <NavButton
          onClick={() => dispatch({ key: AppActionKey.ShowAppSigning })}
          icon={<SignatureIcon />}
          text="App Signing"
          active={state.key === AppStateKey.AppSigning}
          hidden={hideMenuItems}
        />
        <NavButton
          onClick={() => dispatch({ key: AppActionKey.ShowGenerateSwarmKey })}
          icon={<KeyIcon />}
          text="Swarm Key"
          active={state.key === AppStateKey.SwarmKey}
          hidden={hideMenuItems}
        />
        <NavButton
          onClick={() => dispatch({ key: AppActionKey.ShowDiagnostics })}
          icon={<BarChartIcon />}
          text="Diagnostics"
          active={state.key === AppStateKey.Diagnostics}
          hidden={hideMenuItems}
        />
        <NavButton
          onClick={() => dispatch({ key: AppActionKey.ShowPreferences })}
          icon={<PreferencesIcon />}
          text="Preferences"
          active={state.key === AppStateKey.Preferences}
          hidden={hideMenuItems}
        />
        {/* <NavButton
          onClick={() => {
            getNodesDetails(['localhost'])
          }}
          icon={<PreferencesIcon />}
          text="Preferences"
          hidden={hideMenuItems}
        /> */}
        <NavButton
          onClick={() => dispatch({ key: AppActionKey.ShowAbout })}
          icon={<HelpIcon />}
          text="About"
          hidden={hideMenuItems}
          bottom
        />
      </div>
      <div className="flex flex-col flex-grow overflow-hidden">
        <div className="flex items-center flex-shrink-0 h-16 px-7 border-b border-gray-300 z-10">
          <h1 className="text-lg font-medium" id="Layout_Title">
            {title}
          </h1>
          {input && <Input className="ml-auto" {...input} />}
          {actions &&
            actions.length > 0 &&
            actions.map((a, i) => (
              <Action key={a.text + i} className={clsx({ 'ml-auto': !input && i === 0 })} {...a} />
            ))}
          <Menu className={clsx({ 'ml-auto': !input && !actions })} {...menuProps} />
        </div>
        <div className="flex-grow p-4 max-w-full overflow-auto overflow-x-auto flex-shrink bg-gray-200">
          {children}
        </div>
      </div>
    </div>
  )
}
