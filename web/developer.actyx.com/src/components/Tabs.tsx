import React from 'react'
import ThemeTabs from '@theme/Tabs'
import TabItem from '@theme/TabItem'

// A little bit hacky, but hey

// Use like this '<Tabs dontGroup>' to not group
export const Tabs: React.FC<{ dontGroup?: boolean }> = ({ children, dontGroup }) => {
  const values: { label: string; value: string }[] = []

  React.Children.forEach(children, (child) => {
    if (!React.isValidElement(child)) return
    if (child.props.mdxType === 'Windows') {
      values.push({ label: 'Windows', value: 'Windows' })
    }
    if (child.props.mdxType === 'Mac') {
      values.push({ label: 'macOS', value: 'Mac' })
    }
    if (child.props.mdxType === 'Linux') {
      values.push({ label: 'Linux', value: 'Linux' })
    }
    if (child.props.mdxType === 'Android') {
      values.push({ label: 'Android', value: 'Android' })
    }
    if (child.props.mdxType === 'Docker') {
      values.push({ label: 'Docker', value: 'Docker' })
    }
    if (child.props.mdxType === 'JsNode') {
      values.push({ label: 'Javascript (Node.Js)', value: 'JsNode' })
    }
    if (child.props.mdxType === 'TsNode') {
      values.push({ label: 'Typescript (Node.Js)', value: 'TsNode' })
    }
    if (child.props.mdxType === 'JsBrowser') {
      values.push({ label: 'Javascript (Browser)', value: 'JsBrowser' })
    }
    if (child.props.mdxType === 'TsBrowser') {
      values.push({ label: 'Typescript (Browser)', value: 'TsBrowser' })
    }
    if (child.props.mdxType === 'CSharp') {
      values.push({ label: 'C#', value: 'CSharp' })
    }
  })
  return (
    <ThemeTabs
      groupId={dontGroup ? null : 'group'}
      defaultValue={values.length > 0 ? values[0].value : null}
      values={values}
    >
      {React.Children.map(children, (child) => {
        if (!React.isValidElement(child)) return null
        return <TabItem value={child.props.mdxType}>{child.props.children}</TabItem>
      })}
    </ThemeTabs>
  )
}
