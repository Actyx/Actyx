import React, { ReactNode } from 'react'
import styled from 'styled-components'
import renderer from './Renderer'
import Markdown from 'react-markdown'

const Wrapper = styled.div`
  display: grid;
  grid-template-columns: [first] 150px [line2] auto [end];
  grid-template-rows: auto;
  margin-top: -12px;
  margin-bottom: -12px;
`
const Left = styled.div`
  grid-column-start: first;
  grid-column-end: line2;
  text-align: left;
  padding-right: 16px;
`

const Right = styled.div`
  grid-column-start: line2;
  grid-column-end: end;
  text-align: left;
  padding-left: 16px;
`

type Props = Readonly<{
  code: string
  description: ReactNode
}>

export const CLIRef: React.FC<Props> = ({ code, description }: Props) => (
  <Wrapper>
    <Left>
      <Markdown renderers={renderer}>{code}</Markdown>
    </Left>
    <Right>{description}</Right>
  </Wrapper>
)
