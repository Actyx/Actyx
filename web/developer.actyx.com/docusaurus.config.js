const remarkCodeImport = require('remark-code-import')

module.exports = {
  title: 'Actyx Developer',
  tagline: 'Documentation, guides and tools for building on the Actyx Platform',
  url: 'https://developer.actyx.com',
  baseUrl: '/',
  favicon: 'img/favicon.ico',
  organizationName: 'Actyx',
  projectName: 'Actyx Developer Docs',
  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'throw',
  plugins: [
    [
      require.resolve('docusaurus-gtm-plugin'),
      {
        id: 'GTM-5PXCMFH',
      },
    ],
    [require.resolve('./src/plugins/analytics'), {}],
    [require.resolve('./src/plugins/cosmos-versions'), {}],
    /*    [
      require.resolve('./src/plugins/arm'),
      {
        releasesYml: './__auto-releases.yml',
      },
    ], */
    [
      'docusaurus-plugin-typedoc',
      {
        id: 'pond',
        entryPoints: ['../../js/pond/src/index.ts'],
        tsconfig: '../../js/pond/tsconfig.json',
        out: 'reference/pond',
        disableSources: true,
        sidebar: {
          sidebarFile: '__pond-sidebar.js',
          fullNames: false,
        },
      },
    ],
  ],
  themeConfig: {
    announcementBar: {
      id: '2.0.0-release',
      content:
        '🤩 We are incredibly excited to announce the release of Actyx 2.0! Read more about it in our release <a href="https://developer.actyx.com/blog/2021/06/18/actyx-2-0-0-release">blog post</a>. 🤩',
      backgroundColor: '#373c40',
      textColor: '#fff',
    },
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
          label: 'Blog',
          activeBasePath: '/blog',
          position: 'right',
          to: '/blog',
        },
        {
          label: 'Forum',
          position: 'right',
          to: 'https://community.actyx.com/',
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
              label: 'Set up a new project',
              to: 'docs/how-to/local-development/set-up-a-new-project',
            },
            {
              label: 'Jump into Actyx Pond',
              to: 'docs/how-to/actyx-pond/getting-started',
            },
            {
              label: 'Package for mobile',
              to: 'docs/how-to/packaging/mobile-apps',
            },
          ],
        },
        {
          title: 'Essential concepts',
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
            {
              label: 'Local First Cooperation',
              to: 'docs/conceptual/local-first-cooperation',
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
          ],
        },
        {
          title: 'For developers',
          items: [
            {
              label: 'Forum',
              to: 'https://community.actyx.com',
            },
            {
              label: 'Discord',
              to: 'https://discord.gg/262yJhc',
            },
            {
              label: 'Blog',
              to: 'blog',
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
              label: 'Team',
              to: 'https://www.actyx.com/about',
            },
            {
              label: 'News',
              to: 'https://www.actyx.com/news',
            },
            {
              label: 'Careers',
              to: 'https://careers.actyx.io/',
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
    algolia: {
      apiKey: 'dee14099c148f0ca14d046428003623a',
      indexName: 'actyx_developer',
      algoliaOptions: {}, // Optional, if provided by Algolia
    },
  },
  presets: [
    [
      '@docusaurus/preset-classic',
      {
        docs: {
          sidebarPath: require.resolve('./sidebars.js'),
          remarkPlugins: [remarkCodeImport],
        },
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
        blog: {
          postsPerPage: 6,
          blogSidebarTitle: 'Our latest posts',
          blogSidebarCount: 0,
        },
      },
    ],
  ],
}
