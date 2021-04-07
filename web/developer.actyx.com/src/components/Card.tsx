import React from 'react'
import styled from 'styled-components'
import { Link } from '../components/Link'
import DLink from '@docusaurus/Link'

const Wrapper = styled.div<{
  color?: string
}>`
  border-radius: 4px;
  border: ${(p) => (p.color == 'white' ? '1px solid #BFC4CA' : 'none')};
  background: ${(p) =>
    p.color == 'green'
      ? '#15BE53'
      : p.color == 'blue'
      ? '#369AFF'
      : p.color == 'purple'
      ? '#635BFF'
      : p.color == 'orange'
      ? '#FF9933'
      : p.color == 'dark'
      ? '#303c4b'
      : p.color == 'white'
      ? '#ffffff'
      : '#f5f5f5'};
  padding-left: 24px;
  padding-right: 24px;
  padding-bottom: 12px;
  padding-top: 18px;
  flex-grow: 1;
  &:hover {
    transform: translateY(-0.1rem);
    transition: background 150ms ease-in-out;
    transition-property: background;
    transition-duration: 150ms;
    transition-timing-function: ease-in-out;
    transition-delay: 0s;
  }
`

const Headline = styled.div<{
  color: string
}>`
  margin-bottom: 8px;
  font-size: 18px;
  font-weight: 600;
  color: ${(p) => (p.color != 'white' ? 'white' : '#303c4b')};
`

const Body = styled.div<{
  color: string
}>`
  font-size: 15px;
  margin-bottom: 16px;
  color: ${(p) => (p.color != 'white' ? 'white' : '#303c4b')};
`

type Props = Readonly<{
  color: string
  headline: string
  body: string
  cta: string
  link: string
}>

export const Card: React.FC<Props> = ({ color, headline, body, cta, link }: Props) => (
  <DLink to={link} style={{ textDecoration: 'none' }}>
    <Wrapper color={color}>
      <Headline color={color}>{headline}</Headline>
      <Body color={color}>{body}</Body>
      <Link title={cta} link={link} color={color} positive={false}></Link>
    </Wrapper>
  </DLink>
)
