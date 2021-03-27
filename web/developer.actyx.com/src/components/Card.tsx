import React from 'react'
import styled from 'styled-components'
import { Link } from '../components/Link'

const Wrapper = styled.div<{
  color?: string
}>`
  border-radius: 4px;
  background: ${(p) =>
    p.color == 'green'
      ? '#15BE53'
      : p.color == 'blue'
      ? '#369AFF'
      : p.color == 'purple'
      ? '#635BFF'
      : p.color == 'orange'
      ? '#FF9933'
      : '#f5f5f5'};
  padding-left: 24px;
  padding-right: 24px;
  padding-bottom: 12px;
  padding-top: 18px;
  flex-grow: 1;
`

const Headline = styled.div`
  margin-bottom: 8px;
  font-size: 18px;
  font-weight: 600;
  color: white;
`

const Body = styled.div`
  font-size: 15px;
  color: white;
  margin-bottom: 16px;
`

type Props = Readonly<{
  color: string
  headline: string
  body: string
  cta: string
  link: string
}>

export const Card: React.FC<Props> = ({ color, headline, body, cta, link }: Props) => (
  <Wrapper color={color}>
    <Headline>{headline}</Headline>
    <Body>{body}</Body>
    <Link title={cta} link={link} color="white"></Link>
  </Wrapper>
)
