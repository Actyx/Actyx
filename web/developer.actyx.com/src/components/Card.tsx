import React from 'react'
import styled from 'styled-components'
import { Link } from '../components/Link'
import DLink from '@docusaurus/Link'
import defaults from '../components/defaults'

const Wrapper = styled.div<{
  color?: string
}>`
  border-radius: 4px;
  border: ${(p) => (p.color == 'white' ? '1px solid #BFC4CA' : 'none')};
  background: ${(p) =>
    p.color == 'green'
      ? defaults.colors.green
      : p.color == 'blue'
      ? defaults.colors.blue
      : p.color == 'purple'
      ? defaults.colors.purple
      : p.color == 'orange'
      ? defaults.colors.orange
      : p.color == 'dark'
      ? defaults.colors.darkgray
      : p.color == 'white'
      ? defaults.colors.white
      : defaults.colors.lightgray};
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
  color: ${(p) => (p.color != 'white' ? defaults.colors.white : defaults.colors.darkgray)};
`

const Body = styled.div<{
  color: string
}>`
  font-size: 15px;
  margin-bottom: 16px;
  color: ${(p) => (p.color != 'white' ? defaults.colors.white : defaults.colors.darkgray)};
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
