const remarkCodeImport = require('remark-code-import')

module.exports = {
  title: 'Actyx Developer',
  tagline: 'Documentation, guides and tools for building on the Actyx Platform',
  url: 'https://developer.actyx.com',
  baseUrl: '/',
  trailingSlash: false,
  favicon: 'img/favicon.ico',
  organizationName: 'Actyx',
  projectName: 'Actyx Developer Docs',
  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'throw',
  markdown: {
    mermaid: true
  },
  plugins: [
    "@docusaurus/theme-mermaid",
    [
      require.resolve('docusaurus-gtm-plugin'),
      {
        id: 'GTM-5PXCMFH',
      },
    ],
    [require.resolve('./src/plugins/cosmos-versions'), {}],
    [require.resolve('docusaurus-lunr-search'), {}],
  ],
  themeConfig: {
    //announcementBar: {
    //  id: '2.0.0-release',
    //  content:
    //    '🤩 We are incredibly excited to announce the release of Actyx 2.0! Read more about it in our release <a href="https://developer.actyx.com/blog/2021/06/23/actyx-2-0-0-release">blog post</a>. 🤩',
    //  backgroundColor: '#373c40',
    //  textColor: '#fff',
    //},
    colorMode: {
      defaultMode: 'light',
      disableSwitch: true,
      respectPrefersColorScheme: false,
    },
    navbar: {
      title: '',
      logo: {
        alt: 'Actyx Developer',
        src: 'img/header.svg',
      },
      items: [
        {
          label: 'How-to Guides',
          activeBasePath: 'docs/how-to',
          to: 'docs/how-to/overview',
        },
        {
          label: 'Conceptual Guides',
          activeBasePath: 'docs/conceptual',
          to: 'docs/conceptual/overview',
        },
        {
          label: 'Reference',
          activeBasePath: 'docs/reference',
          to: 'docs/reference/overview',
        },
        {
          label: 'Tutorials',
          activeBasePath: 'docs/tutorials',
          to: 'docs/tutorials/overview',
        },
        {
          label: 'Releases',
          activeBasePath: '/releases',
          position: 'right',
          to: '/releases',
        },
        {
          label: 'Chat',
          position: 'right',
          to: 'https://discord.gg/4RZpTqmPgC',
        },
        {
          label: 'Forum',
          position: 'right',
          to: 'https://groups.google.com/a/actyx.io/g/developers',
        },
      ],
    },
    footer: {
      logo: {
        alt: 'Actyx Developer',
        src: 'img/header.svg',
        href: 'https://developer.actyx.com',
      },
      style: 'dark',
      links: [
        {
          title: 'Get started',
          items: [
            {
              label: 'Install and start Actyx',
              to: 'docs/how-to/local-development/install-actyx',
            },
            {
              label: 'Jump into Actyx',
              to: 'docs/how-to/overview#business-logic',
            },
            {
              label: 'Package for mobile',
              to: 'docs/how-to/packaging/mobile-apps',
            },
          ],
        },
        {
          title: 'Concepts',
          items: [
            {
              label: 'How Actyx works',
              to: 'docs/conceptual/how-actyx-works',
            },
            {
              label: 'Event-based systems',
              to: 'docs/conceptual/event-sourcing',
            },
            {
              label: 'Actyx jargon',
              to: 'docs/conceptual/actyx-jargon',
            },
          ],
        },
        {
          title: 'Reference docs',
          items: [
            {
              label: 'Actyx',
              to: 'docs/reference/actyx-reference',
            },
            {
              label: 'CLI',
              to: 'docs/reference/cli/cli-overview',
            },
            {
              label: 'Node Manager',
              to: 'docs/reference/node-manager',
            },
            {
              label: 'Actyx Query Language',
              to: 'docs/reference/aql',
            },
          ],
        },
        {
          title: 'Company',
          items: [
            {
              label: 'Home',
              to: 'https://www.actyx.com',
            },
            {
              label: 'Chat',
              to: 'https://discord.gg/4RZpTqmPgC',
            },
            {
              label: 'Forum',
              to: 'https://groups.google.com/a/actyx.io/g/developers',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Actyx AG`,
    },
    prism: {
      theme: require('prism-react-renderer/themes/vsDark'),
      darkTheme: require('prism-react-renderer/themes/dracula'),
      additionalLanguages: ['rust', 'csharp'],
    },
  },
  presets: [
    [
      '@docusaurus/preset-classic',
      {
        docs: {
          sidebarPath: require.resolve('./sidebars.js'),
          remarkPlugins: [remarkCodeImport],
          editUrl: 'https://github.com/Actyx/Actyx/tree/master/web/developer.actyx.com',
          editCurrentVersion: true,
          showLastUpdateAuthor: true,
          showLastUpdateTime: true,
        },
        //theme: {
        //  customCss: require.resolve('./src/css/custom.css'),
        //},
        blog: {
          postsPerPage: 6,
          blogSidebarTitle: 'Our latest posts',
          blogSidebarCount: 0,
        },
      },
    ],
  ],
}
