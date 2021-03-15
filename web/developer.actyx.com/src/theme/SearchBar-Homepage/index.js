/**
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import React, { useState, useRef, useCallback, useMemo } from 'react'
import { createPortal } from 'react-dom'
import useDocusaurusContext from '@docusaurus/useDocusaurusContext'
import { useHistory } from '@docusaurus/router'
import { useBaseUrlUtils } from '@docusaurus/useBaseUrl'
import Link from '@docusaurus/Link'
import Head from '@docusaurus/Head'
import useSearchQuery from '@theme/hooks/useSearchQuery'
import { DocSearchButton, useDocSearchKeyboardEvents } from '@docsearch/react'
import useAlgoliaContextualFacetFilters from '@theme/hooks/useAlgoliaContextualFacetFilters'

let DocSearchModal = null

function Hit({ hit, children }) {
  return <Link to={hit.url}>{children}</Link>
}

function ResultsFooter({ state, onClose }) {
  const { generateSearchPageLink } = useSearchQuery()

  return (
    <Link to={generateSearchPageLink(state.query)} onClick={onClose}>
      See all {state.context.nbHits} results
    </Link>
  )
}

function DocSearch({ contextualSearch, ...props }) {
  const { siteMetadata } = useDocusaurusContext()

  const contextualSearchFacetFilters = useAlgoliaContextualFacetFilters()

  const configFacetFilters = props.searchParameters?.facetFilters ?? []

  const facetFilters = contextualSearch
    ? // Merge contextual search filters with config filters
      [...contextualSearchFacetFilters, ...configFacetFilters]
    : // ... or use config facetFilters
      configFacetFilters

  // we let user override default searchParameters if he wants to
  const searchParameters = {
    ...props.searchParameters,
    facetFilters,
  }

  const { withBaseUrl } = useBaseUrlUtils()
  const history = useHistory()
  const searchButtonRef = useRef(null)
  const [isOpen, setIsOpen] = useState(false)
  const [initialQuery, setInitialQuery] = useState(null)

  const importDocSearchModalIfNeeded = useCallback(() => {
    if (DocSearchModal) {
      return Promise.resolve()
    }

    return Promise.all([
      import('@docsearch/react/modal'),
      import('@docsearch/react/style'),
      import('./styles.css'),
    ]).then(([{ DocSearchModal: Modal }]) => {
      DocSearchModal = Modal
    })
  }, [])

  const onOpen = useCallback(() => {
    importDocSearchModalIfNeeded().then(() => {
      setIsOpen(true)
    })
  }, [importDocSearchModalIfNeeded, setIsOpen])

  const onClose = useCallback(() => {
    setIsOpen(false)
  }, [setIsOpen])

  const onInput = useCallback(
    (event) => {
      importDocSearchModalIfNeeded().then(() => {
        setIsOpen(true)
        setInitialQuery(event.key)
      })
    },
    [importDocSearchModalIfNeeded, setIsOpen, setInitialQuery],
  )

  const navigator = useRef({
    navigate({ itemUrl }) {
      history.push(itemUrl)
    },
  }).current

  const transformItems = useRef((items) => {
    return items.map((item) => {
      // We transform the absolute URL into a relative URL.
      // Alternatively, we can use `new URL(item.url)` but it's not
      // supported in IE.
      const a = document.createElement('a')
      a.href = item.url

      return {
        ...item,
        url: withBaseUrl(`${a.pathname}${a.hash}`),
      }
    })
  }).current

  const resultsFooterComponent = useMemo(
    () => (footerProps) => <ResultsFooter {...footerProps} onClose={onClose} />,
    [onClose],
  )

  const cssStyle = {
    width: '600px',
    height: '50px',
    background: 'white',
    borderRadius: '5px',
    border: '1px solid #e1e5e8',
    marginLeft: '24px',
    marginRight: '24px',
    marginTop: '32px',
  }
  const transformSearchClient = useCallback(
    (searchClient) => {
      searchClient.addAlgoliaAgent('docusaurus', siteMetadata.docusaurusVersion)

      return searchClient
    },
    [siteMetadata.docusaurusVersion],
  )

  useDocSearchKeyboardEvents({
    isOpen,
    onOpen,
    onClose,
    onInput,
    searchButtonRef,
  })

  return (
    <>
      <Head>
        {/* This hints the browser that the website will load data from Algolia,
        and allows it to preconnect to the DocSearch cluster. It makes the first
        query faster, especially on mobile. */}
        <link
          rel="preconnect"
          href={`https://${props.appId}-dsn.algolia.net`}
          crossOrigin="anonymous"
        />
      </Head>

      <DocSearchButton
        onTouchStart={importDocSearchModalIfNeeded}
        onFocus={importDocSearchModalIfNeeded}
        onMouseOver={importDocSearchModalIfNeeded}
        onClick={onOpen}
        ref={searchButtonRef}
        style={cssStyle}
      />

      {isOpen &&
        createPortal(
          <DocSearchModal
            onClose={onClose}
            initialScrollY={window.scrollY}
            initialQuery={initialQuery}
            navigator={navigator}
            transformItems={transformItems}
            hitComponent={Hit}
            resultsFooterComponent={resultsFooterComponent}
            transformSearchClient={transformSearchClient}
            {...props}
            searchParameters={searchParameters}
          />,
          document.body,
        )}
    </>
  )
}

function SearchBarHomePage() {
  const { siteConfig } = useDocusaurusContext()
  return <DocSearch {...siteConfig.themeConfig.algolia} />
}

export default SearchBarHomePage
