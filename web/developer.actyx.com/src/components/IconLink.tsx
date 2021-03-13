import React from 'react'
import styled from 'styled-components'
import { Arrow } from '../icons/icons'

const Wrapper = styled.div`
  display: flex;
  flex-direction: row;
  margin-bottom: 24px;
`

const Link = styled.div`
  color: #ebedf0;
  margin-right: 4px;
  font-size: 15px;
`

type Props = Readonly<{
  link: string
}>

export const IconLink: React.FC<Props> = ({ link }: Props) => (
  <a href="/releases">
    <Wrapper>
      <Link>{link}</Link>
      <Arrow />
    </Wrapper>
  </a>
)
