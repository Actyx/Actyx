/**
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
import React from 'react'
import clsx from 'clsx'
import Link from '@docusaurus/Link'
import { useThemeConfig } from '@docusaurus/theme-common'
import useBaseUrl from '@docusaurus/useBaseUrl'
import { BuildNumber } from '../../components/BuildNumber'
import { IconLink } from '../../components/IconLink'
import { Social } from '../../components/Social'

function FooterLink({ to, href, label, prependBaseUrlToHref, ...props }) {
  const toUrl = useBaseUrl(to)
  const normalizedHref = useBaseUrl(href, {
    forcePrependBaseUrl: true,
  })
  return (
    <Link
      className="footer__link-item"
      {...(href
        ? {
            target: '_blank',
            rel: 'noopener noreferrer',
            href: prependBaseUrlToHref ? normalizedHref : href,
          }
        : {
            to: toUrl,
          })}
      {...props}
    >
      {label}
    </Link>
  )
}

function Footer() {
  const { footer } = useThemeConfig()
  const { copyright, links = [], logo = {} } = footer || {}

  if (!footer) {
    return null
  }

  return (
    <footer
      className={clsx('footer', {
        'footer--dark': footer.style === 'dark',
      })}
    >
      <div className="container">
        {links && links.length > 0 && (
          <div className="row footer__links">
            {links.map((linkItem, i) => (
              <div key={i} className="col footer__col">
                {linkItem.title != null ? (
                  <h4 className="footer__title">{linkItem.title}</h4>
                ) : null}
                {linkItem.items != null &&
                Array.isArray(linkItem.items) &&
                linkItem.items.length > 0 ? (
                  <ul className="footer__items">
                    {linkItem.items.map((item, key) =>
                      item.html ? (
                        <li
                          key={key}
                          className="footer__item" // Developer provided the HTML, so assume it's safe.
                          // eslint-disable-next-line
                          dangerouslySetInnerHTML={{
                            __html: item.html,
                          }}
                        />
                      ) : (
                        <li key={item.href || item.to} className="footer__item">
                          <FooterLink {...item} />
                        </li>
                      ),
                    )}
                  </ul>
                ) : null}
              </div>
            ))}
          </div>
        )}
        <div style={{ paddingLeft: '84px' }}>
          <BuildNumber pre="Release: " build="build-1.17242" />{' '}
          {/* TODO get latest release build */}
          <IconLink link="Past Releases" />
          <Social />
          {(logo || copyright) && (
            <div className="footer__bottom">
              {copyright ? (
                <div
                  className="footer__copyright" // Developer provided the HTML, so assume it's safe.
                  // eslint-disable-next-line
                  dangerouslySetInnerHTML={{
                    __html: copyright,
                  }}
                />
              ) : null}
            </div>
          )}
        </div>
      </div>
    </footer>
  )
}

export default Footer
