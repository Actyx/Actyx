import React from 'react'
import styled from 'styled-components'
import renderer from './Renderer'
import Markdown from 'react-markdown'

const Wrapper = styled.div`
  border-radius: 8px;
  max-width: 800px;
  min-height: 92px;
  background: #f5f6f7;
  border: 1px solid #eef1f5;
  padding: 18px;
  display: grid;
  grid-template-columns: [first] 32px [line2] auto [end];
  grid-template-rows: [row1-start] 32px [row1-end] auto [row2-end];
`
const Icon = styled.div`
  font-size: 20px;
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
  padding-top: 3px;
`

const Body = styled.div`
  grid-column-start: line2;
  grid-column-end: end;
  grid-row-start: row1-end;
  grid-row-end: row2-end;
`

type Props = Readonly<{
  icon: string
  headline: string
  body: string
}>

export const Callout: React.FC<Props> = ({ icon, headline, body }: Props) => (
  <Wrapper>
    <Icon>{icon}</Icon>
    <Headline>
      <Markdown renderers={renderer}>{headline}</Markdown>
    </Headline>
    <Body>
      <Markdown renderers={renderer}>{body}</Markdown>
    </Body>
  </Wrapper>
)
