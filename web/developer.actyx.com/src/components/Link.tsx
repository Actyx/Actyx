import React from 'react'
import styled from 'styled-components'
import { ArrowBlue } from '../icons/icons'

const Wrapper = styled.div`
  display: flex;
  flex-direction: row;
  margin-bottom: 6px;
`

const LinkContent = styled.div`
  color: #1998ff;
  margin-right: 4px;
  font-size: 15px;
`

type Props = Readonly<{
  title: string
  link: string
}>

export const Link: React.FC<Props> = ({ title, link }: Props) => (
  <a href={link}>
    <Wrapper>
      <LinkContent>{title}</LinkContent>
      <ArrowBlue />
    </Wrapper>
  </a>
)
