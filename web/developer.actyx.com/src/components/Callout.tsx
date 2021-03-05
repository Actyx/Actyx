import React from 'react'
import styled from 'styled-components'

const Wrapper = styled.div`
  border-radius: 12px;
  max-width: 700px;
  min-height: 92px;
  background: #f5f6f7;
  border: 1px solid #eef1f5;
  padding: 22px;
  display: grid;
  grid-template-columns: [first] 32px [line2] auto [end];
  grid-template-rows: [row1-start] 32px [row1-end] auto [row2-end];
`
const Icon = styled.div`
  grid-column-start: first;
  grid-column-end: line2;
  grid-row-start: row1-start;
  grid-row-end: row1-end;
`

const Headline = styled.div`
  grid-column-start: line2;
  grid-column-end: end;
  grid-row-start: row1-start;
  grid-row-end: row1-end;
  font-weight: 700;
`

const Body = styled.div`
  grid-column-start: line2;
  grid-column-end: end;
  grid-row-start: row1-end;
  grid-row-end: row2-end;
`

type Props = Readonly<{
  example: string
}>

export const Callout: React.FC<Props> = ({ example }: Props) => (
  <Wrapper>
    <Icon>🔥</Icon>
    <Headline>No help needed?</Headline>
    <Body>
      If you already know how to install command line tools, you can directly go to our downloads
      page and download the binary file for the Actyx CLI
    </Body>
  </Wrapper>
)
