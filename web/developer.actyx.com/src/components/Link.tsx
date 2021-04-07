import React from 'react'
import styled from 'styled-components'
import { Arrow } from '../icons/icons'
import DLink from '@docusaurus/Link'

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
      ? '#15BE53'
      : p.color == 'blue' && p.positive
      ? '#369AFF'
      : p.color == 'purple' && p.positive
      ? '#635BFF'
      : p.color == 'orange' && p.positive
      ? '#FF9933'
      : p.color == 'dark' && p.positive
      ? '#303c4b'
      : p.color == 'white'
      ? '#303c4b'
      : '#ffffff'};
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
