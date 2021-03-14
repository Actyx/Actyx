import React from 'react'
import styled from 'styled-components'

const Wrapper = styled.div`
  width: 100%;
  height: 1px;
  border-bottom: 1px solid #ebedf0;
  margin-top: 48px;
  margin-bottom: 48px;
`

export const Divider: React.FC = () => <Wrapper></Wrapper>
