import React from 'react'
import styled from 'styled-components'

const Wrapper = styled.div`
  font-size: 18px;
  color: #606770;
  margin-bottom: 32px;
`

type Props = Readonly<{
  content: string
}>

export const Subheadline: React.FC<Props> = ({ content }: Props) => <Wrapper>{content}</Wrapper>
