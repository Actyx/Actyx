import React from 'react'
import CodeBlock from '@theme/CodeBlock'
import Mermaid from '@theme/Mermaid'
import Details from '@theme/Details'

export const ExpandableDiagram = ({
  code,
  title,
  presentedCode,
}: {
  code: string
  title?: string
  presentedCode?: string
}): JSX.Element => (
  <>
    <CodeBlock language="text" title={title}>
      {presentedCode !== null ? presentedCode : code}
    </CodeBlock>
    <Details
      summary={
        <summary>
          <strong>Expand to see diagram</strong>
        </summary>
      }
    >
      <Mermaid value={code} />
    </Details>
  </>
)

// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace ExpandableDiagramUtils {
  export const indent = (code: string, indentation: number): string =>
    code
      .split('\n')
      .map((line) => `${new Array(indentation).fill(' ').join('')}${line}`)
      .join('\n')
}
