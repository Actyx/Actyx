import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism'
import React from 'react'

const renderers = {
  code: ({ language, value }) => {
    return <SyntaxHighlighter style={vscDarkPlus} language={language} children={value} />
  },
}

export default renderers
