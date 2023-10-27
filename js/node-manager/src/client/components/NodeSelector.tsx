import { NodeType, ReachableNodeUi } from 'common/types/nodes'
import React, { useEffect } from 'react'
import Select from 'react-select'
import semver from 'semver'

export const NodeSelector = ({
  nodes,
  selectedNodeAddr,
  onChange,
}: {
  nodes: ReachableNodeUi[]
  selectedNodeAddr: string | undefined
  onChange: (val: { label: string; value: string; disabled: boolean } | null) => unknown
}) => {
  const options = nodes.map((n) => {
    const opt = { value: n.addr }
    if (n.type !== NodeType.Reachable) {
      return {
        ...opt,
        label: `${n.addr}: node not reachable`,
        disabled: true,
      }
    }

    const version = semver.coerce(n.details.version)
    if (!semver.valid(version) || version === null || !semver.satisfies(version, '>=2.2.0')) {
      return {
        ...opt,
        label: `${n.details.displayName} (${n.addr}): not supported; upgrade to Actyx 2.2.0 or above`,
        disabled: true,
      }
    }
    return {
      ...opt,
      label: `${n.details.displayName} (${n.addr})`,
      disabled: false,
    }
  })

  const defaultOption = options.find(
    (o) => o.value === selectedNodeAddr && o.disabled === false,
  )?.label

  useEffect(() => {
    // Check if recently selected node is no longer available
    if (selectedNodeAddr === undefined) return
    const defaultNode = nodes.find((node) => node.addr === selectedNodeAddr)
    if (defaultNode === undefined) {
      onChange(null)
    }
  }, [selectedNodeAddr, nodes])

  return (
    <Select
      options={options}
      isOptionDisabled={(o) => !!o.disabled}
      placeholder="Select node..."
      onChange={(v) => onChange(v)}
      className="flex-grow bg-white"
      defaultInputValue={defaultOption}
    />
  )
}
