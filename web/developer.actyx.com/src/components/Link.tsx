import React from 'react'
import styled from 'styled-components'
import { Arrow } from '../icons/icons'
import DLink from '@docusaurus/Link'
import defaults from '../components/defaults'

const Wrapper = styled.div`
  display: flex;
  flex-direction: row;
  margin-bottom: 6px;
`

const LinkContent = styled.div<{
  color: string
  positive: boolean
}>`
  color: ${(p) =>
    p.color == 'green' && p.positive
      ? defaults.colors.green
      : p.color == 'blue' && p.positive
      ? defaults.colors.blue
      : p.color == 'purple' && p.positive
      ? defaults.colors.purple
      : p.color == 'orange' && p.positive
      ? defaults.colors.orange
      : p.color == 'dark' && p.positive
      ? defaults.colors.darkgray
      : p.color == 'white'
      ? defaults.colors.darkgray
      : defaults.colors.white};
  margin-right: 4px;
  font-size: 15px;
  font-weight: 600;
`

type Props = Readonly<{
  title: string
  link: string
  color: string
  positive: boolean
}>

export const Link: React.FC<Props> = ({ title, link, color, positive }: Props) => (
  <DLink to={link}>
    <Wrapper>
      <LinkContent positive={positive} color={color}>
        {title}
      </LinkContent>
      <Arrow color={color} positive={positive} />
    </Wrapper>
  </DLink>
)
