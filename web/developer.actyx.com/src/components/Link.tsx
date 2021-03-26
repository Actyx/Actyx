import React from 'react'
import styled from 'styled-components'
import { ArrowBlue } from '../icons/icons'

const Wrapper = styled.div`
  display: flex;
  flex-direction: row;
  margin-bottom: 6px;
`

const LinkContent = styled.div<{
  color: string
}>`
  color: ${(p) =>
    p.color == 'green'
      ? '#15BE53'
      : p.color == 'blue'
      ? '#369AFF'
      : p.color == 'purple'
      ? '#635BFF'
      : p.color == 'orange'
      ? '#FF9933'
      : p.color == 'white'
      ? '#ffffff'
      : '#f5f5f5'};
  margin-right: 4px;
  font-size: 15px;
  font-weight: 600;
`

type Props = Readonly<{
  title: string
  link: string
  color: string
}>

export const Link: React.FC<Props> = ({ title, link, color }: Props) => (
  <a href={link}>
    <Wrapper>
      <LinkContent color={color}>{title}</LinkContent>
    </Wrapper>
  </a>
)
