import React from 'react'
import styled from 'styled-components'

const Wrapper = styled.div`
  font-size: 15px;
  color: #5d6979;
  margin-bottom: 16px;
`

type Props = Readonly<{
  body: string
}>

export const Body: React.FC<Props> = ({ body }: Props) => <Wrapper>{body}</Wrapper>
