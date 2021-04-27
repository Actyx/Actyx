import React from 'react'
import styled from 'styled-components'
import { ReactTypeformEmbed } from 'react-typeform-embed'

const Wrapper = styled.div`
  position: relative;
  heigth: 400px;
`

export const NPS: React.FC = () => (
  <Wrapper>
    <ReactTypeformEmbed
      popup={false}
      url="https://form.typeform.com/to/dHlLetfi"
      style={{
        height: '400px',
        position: 'relative',
      }}
    />
    ;
  </Wrapper>
)

export default NPS
